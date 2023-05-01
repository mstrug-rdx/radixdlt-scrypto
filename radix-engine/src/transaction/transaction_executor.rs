use crate::blueprints::transaction_processor::TransactionProcessorError;
use crate::errors::*;
use crate::kernel::id_allocator::IdAllocator;
use crate::kernel::kernel::KernelBoot;
use crate::system::module_mixer::SystemModuleMixer;
use crate::system::system_callback::SystemConfig;
use crate::system::system_modules::costing::*;
use crate::track::{to_state_updates, Track};
use crate::transaction::*;
use crate::types::*;
use crate::vm::wasm::*;
use crate::vm::{ScryptoVm, Vm};
use radix_engine_constants::*;
use radix_engine_interface::api::LockFlags;
use radix_engine_interface::blueprints::resource::LiquidFungibleResource;
use radix_engine_interface::blueprints::transaction_processor::{
    InstructionOutput, TransactionProcessorRunInput,
};
use radix_engine_interface::blueprints::transaction_processor::{
    TRANSACTION_PROCESSOR_BLUEPRINT, TRANSACTION_PROCESSOR_RUN_IDENT,
};
use radix_engine_stores::interface::*;
use radix_engine_stores::jmt_support::JmtMapper;
use sbor::rust::borrow::Cow;
use transaction::model::*;

pub struct FeeReserveConfig {
    pub cost_unit_price: u128,
    pub system_loan: u32,
}

impl Default for FeeReserveConfig {
    fn default() -> Self {
        FeeReserveConfig::standard()
    }
}

impl FeeReserveConfig {
    pub fn standard() -> Self {
        Self {
            cost_unit_price: DEFAULT_COST_UNIT_PRICE,
            system_loan: DEFAULT_SYSTEM_LOAN,
        }
    }
}

pub struct ExecutionConfig {
    pub genesis: bool,
    pub kernel_trace: bool,
    pub execution_trace: Option<usize>,
    pub max_call_depth: usize,
    pub abort_when_loan_repaid: bool,
    pub max_wasm_mem_per_transaction: usize,
    pub max_wasm_mem_per_call_frame: usize,
    pub max_substate_reads_per_transaction: usize,
    pub max_substate_writes_per_transaction: usize,
    pub max_substate_size: usize,
    pub max_invoke_input_size: usize,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        ExecutionConfig::standard()
    }
}

impl ExecutionConfig {
    pub fn standard() -> Self {
        Self {
            genesis: false,
            kernel_trace: false,
            execution_trace: Some(1),
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            abort_when_loan_repaid: false,
            max_wasm_mem_per_transaction: DEFAULT_MAX_WASM_MEM_PER_TRANSACTION,
            max_wasm_mem_per_call_frame: DEFAULT_MAX_WASM_MEM_PER_CALL_FRAME,
            max_substate_reads_per_transaction: DEFAULT_MAX_SUBSTATE_READS_PER_TRANSACTION,
            max_substate_writes_per_transaction: DEFAULT_MAX_SUBSTATE_WRITES_PER_TRANSACTION,
            max_substate_size: DEFAULT_MAX_SUBSTATE_SIZE,
            max_invoke_input_size: DEFAULT_MAX_INVOKE_INPUT_SIZE,
        }
    }

    pub fn genesis() -> Self {
        Self {
            genesis: true,
            ..Self::default()
        }
    }

    pub fn with_trace(mut self, kernel_trace: bool) -> Self {
        self.kernel_trace = kernel_trace;
        self
    }

    pub fn up_to_loan_repayment() -> Self {
        Self {
            abort_when_loan_repaid: true,
            ..Self::default()
        }
    }

    pub fn up_to_loan_repayment_with_debug() -> Self {
        Self {
            abort_when_loan_repaid: true,
            kernel_trace: true,
            ..Self::default()
        }
    }
}

/// An executor that runs transactions.
/// This is no longer public -- it can be removed / merged into the exposed functions in a future small PR
/// But I'm not doing it in this PR to avoid merge conflicts in the body of execute_with_fee_reserve
struct TransactionExecutor<'s, 'w, S, W>
where
    S: SubstateDatabase,
    W: WasmEngine,
{
    substate_db: &'s S,
    scrypto_vm: &'w ScryptoVm<W>,
}

impl<'s, 'w, S, W> TransactionExecutor<'s, 'w, S, W>
where
    S: SubstateDatabase,
    W: WasmEngine,
{
    pub fn new(substate_db: &'s S, scrypto_vm: &'w ScryptoVm<W>) -> Self {
        Self {
            substate_db,
            scrypto_vm,
        }
    }

    pub fn execute(
        &mut self,
        transaction: &Executable,
        fee_reserve_config: &FeeReserveConfig,
        execution_config: &ExecutionConfig,
    ) -> TransactionReceipt {
        let fee_reserve = match transaction.fee_payment() {
            FeePayment::User {
                cost_unit_limit,
                tip_percentage,
            } => SystemLoanFeeReserve::new(
                fee_reserve_config.cost_unit_price,
                *tip_percentage,
                *cost_unit_limit,
                fee_reserve_config.system_loan,
                execution_config.abort_when_loan_repaid,
            ),
            FeePayment::NoFee => SystemLoanFeeReserve::no_fee(),
        };

        self.execute_with_fee_reserve(transaction, execution_config, fee_reserve, FeeTable::new())
    }

    fn execute_with_fee_reserve(
        &mut self,
        executable: &Executable,
        execution_config: &ExecutionConfig,
        fee_reserve: SystemLoanFeeReserve,
        fee_table: FeeTable,
    ) -> TransactionReceipt {
        #[cfg(not(feature = "alloc"))]
        if execution_config.kernel_trace {
            println!("{:-^80}", "Transaction Metadata");
            println!("Transaction hash: {}", executable.transaction_hash());
            println!(
                "Preallocated Node IDs: {:?}",
                executable.pre_allocated_ids()
            );
            println!("Number of blobs: {}", executable.blobs().len());

            println!("{:-^80}", "Engine Execution Log");
        }

        // Start resources usage measurement
        #[cfg(all(target_os = "linux", feature = "std", feature = "cpu_ram_metrics"))]
        let mut resources_tracker =
            crate::kernel::resources_tracker::ResourcesTracker::start_measurement();

        // Prepare
        let mut track = Track::<_, JmtMapper>::new(self.substate_db);
        let mut id_allocator = IdAllocator::new(
            executable.transaction_hash().clone(),
            executable.pre_allocated_ids().clone(),
        );
        let mut system = SystemConfig {
            callback_obj: Vm {
                scrypto_vm: self.scrypto_vm,
            },
            modules: SystemModuleMixer::standard(
                executable.transaction_hash().clone(),
                executable.auth_zone_params().clone(),
                fee_reserve,
                fee_table,
                executable.payload_size(),
                executable.auth_zone_params().initial_proofs.len(),
                execution_config,
            ),
        };

        let kernel_boot = KernelBoot {
            id_allocator: &mut id_allocator,
            callback: &mut system,
            store: &mut track,
        };

        let invoke_result = kernel_boot
            .call_function(
                TRANSACTION_PROCESSOR_PACKAGE,
                TRANSACTION_PROCESSOR_BLUEPRINT,
                TRANSACTION_PROCESSOR_RUN_IDENT,
                scrypto_encode(&TransactionProcessorRunInput {
                    transaction_hash: executable.transaction_hash().clone(),
                    runtime_validations: executable.runtime_validations().clone(),
                    instructions: executable.instructions().clone(),
                    blobs: executable
                        .blobs()
                        .into_iter()
                        .map(|(hash, blob)| (*hash, Cow::from(*blob)))
                        .collect(),
                    references: executable.references().clone(),
                })
                .unwrap(),
            )
            .map(|rtn| {
                let output: Vec<InstructionOutput> = scrypto_decode(&rtn).unwrap();
                output
            });

        let mut fee_reserve = system.modules.costing.fee_reserve();
        let mut application_events = system.modules.events.events();
        let application_logs = system.modules.logger.logs();

        // Finalize
        let result_type = determine_result_type(invoke_result, &mut fee_reserve);
        let transaction_result = match result_type {
            TransactionResultType::Commit(outcome) => {
                let is_success = outcome.is_ok();

                // Commit/revert
                if !is_success {
                    fee_reserve.revert_royalty();
                    application_events.clear();
                    // application logs retain
                    track.revert_non_force_write_changes();
                }

                // Finalize fees
                let (fee_summary, fee_payments) =
                    distribute_fees(&mut track, fee_reserve, is_success);

                // Finalize track
                let tracked_nodes = track.finalize();
                let state_update_summary =
                    StateUpdateSummary::new(self.substate_db, &tracked_nodes);
                let track_updates = to_state_updates::<JmtMapper>(tracked_nodes);

                TransactionResult::Commit(CommitResult {
                    state_updates: track_updates,
                    state_update_summary,
                    outcome: match outcome {
                        Ok(o) => TransactionOutcome::Success(o),
                        Err(e) => TransactionOutcome::Failure(e),
                    },
                    fee_summary,
                    fee_payments,
                    application_events,
                    application_logs,
                })
            }
            TransactionResultType::Reject(error) => {
                TransactionResult::Reject(RejectResult { error })
            }
            TransactionResultType::Abort(error) => {
                TransactionResult::Abort(AbortResult { reason: error })
            }
        };
        let execution_trace = system.modules.execution_trace.finalize(&transaction_result);
        let execution_metrics = system
            .modules
            .transaction_limits
            .finalize(&transaction_result);

        // Finish resources usage measurement and get results
        let resources_usage = match () {
            #[cfg(not(all(target_os = "linux", feature = "std", feature = "cpu_ram_metrics")))]
            () => ResourcesUsage::default(),
            #[cfg(all(target_os = "linux", feature = "std", feature = "cpu_ram_metrics"))]
            () => resources_tracker.end_measurement(),
        };

        // Produce final receipt
        let receipt = TransactionReceipt {
            result: transaction_result,
            execution_trace,
            execution_metrics,
            resources_usage,
        };

        #[cfg(not(feature = "alloc"))]
        if execution_config.kernel_trace {
            TransactionExecutor::<S, W>::print_execution_summary(&receipt);
        }

        receipt
    }

    #[cfg(not(feature = "alloc"))]
    fn print_execution_summary(receipt: &TransactionReceipt) {
        match &receipt.result {
            TransactionResult::Commit(commit) => {
                println!("{:-^80}", "Cost Analysis");
                let break_down = commit
                    .fee_summary
                    .execution_cost_breakdown
                    .iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<BTreeMap<String, &u32>>();
                for (k, v) in break_down {
                    println!("        + {} /* {} */", v, k);
                }

                println!("{:-^80}", "Cost Totals");
                println!(
                    "{:<30}: {:>10}",
                    "Total Cost Units Consumed", commit.fee_summary.execution_cost_sum
                );
                println!(
                    "{:<30}: {:>10}",
                    "Total Royalty Units Consumed", commit.fee_summary.royalty_cost_sum
                );
                println!(
                    "{:<30}: {:>10}",
                    "Cost Unit Limit", commit.fee_summary.cost_unit_limit
                );
                // NB - we use "to_string" to ensure they align correctly
                println!(
                    "{:<30}: {:>10}",
                    "Execution XRD",
                    commit.fee_summary.total_execution_cost_xrd.to_string()
                );
                println!(
                    "{:<30}: {:>10}",
                    "Royalty XRD",
                    commit.fee_summary.total_royalty_cost_xrd.to_string()
                );
                println!("{:-^80}", "Execution Metrics");
                println!(
                    "{:<30}: {:>10}",
                    "Total Substate Read Bytes", receipt.execution_metrics.substate_read_size
                );
                println!(
                    "{:<30}: {:>10}",
                    "Total Substate Write Bytes", receipt.execution_metrics.substate_write_size
                );
                println!(
                    "{:<30}: {:>10}",
                    "Substate Read Count", receipt.execution_metrics.substate_read_count
                );
                println!(
                    "{:<30}: {:>10}",
                    "Substate Write Count", receipt.execution_metrics.substate_write_count
                );
                println!(
                    "{:<30}: {:>10}",
                    "Peak WASM Memory Usage Bytes", receipt.execution_metrics.max_wasm_memory_used
                );
                println!(
                    "{:<30}: {:>10}",
                    "Max Invoke Payload Size Bytes",
                    receipt.execution_metrics.max_invoke_payload_size
                );
                println!("{:-^80}", "Application Logs");
                for (level, message) in &commit.application_logs {
                    println!("[{}] {}", level, message);
                }
                println!("{:-^80}", "Outcome");
                println!(
                    "{}",
                    match &commit.outcome {
                        TransactionOutcome::Success(_) => "Success".to_string(),
                        TransactionOutcome::Failure(error) => format!("Failure: {:?}", error),
                    }
                );
            }
            TransactionResult::Reject(e) => {
                println!("{:-^80}", "Transaction Rejected");
                println!("{:?}", e.error);
            }
            TransactionResult::Abort(e) => {
                println!("{:-^80}", "Transaction Aborted");
                println!("{:?}", e);
            }
        }
        println!("{:-^80}", "Finish");
    }
}

pub fn execute_and_commit_transaction<
    S: SubstateDatabase + CommittableSubstateDatabase,
    W: WasmEngine,
>(
    substate_db: &mut S,
    scrypto_interpreter: &ScryptoVm<W>,
    fee_reserve_config: &FeeReserveConfig,
    execution_config: &ExecutionConfig,
    transaction: &Executable,
) -> TransactionReceipt {
    let receipt = execute_transaction(
        substate_db,
        scrypto_interpreter,
        fee_reserve_config,
        execution_config,
        transaction,
    );
    if let TransactionResult::Commit(commit) = &receipt.result {
        substate_db.commit(&commit.state_updates.database_updates);
    }
    receipt
}

pub fn execute_transaction<S: SubstateDatabase, W: WasmEngine>(
    substate_db: &S,
    scrypto_interpreter: &ScryptoVm<W>,
    fee_reserve_config: &FeeReserveConfig,
    execution_config: &ExecutionConfig,
    transaction: &Executable,
) -> TransactionReceipt {
    TransactionExecutor::new(substate_db, scrypto_interpreter).execute(
        transaction,
        fee_reserve_config,
        execution_config,
    )
}

enum TransactionResultType {
    Commit(Result<Vec<InstructionOutput>, RuntimeError>),
    Reject(RejectionError),
    Abort(AbortReason),
}

fn determine_result_type(
    mut invoke_result: Result<Vec<InstructionOutput>, RuntimeError>,
    fee_reserve: &mut SystemLoanFeeReserve,
) -> TransactionResultType {
    // A `SuccessButFeeLoanNotRepaid` error is issued if a transaction finishes before
    // the SYSTEM_LOAN_AMOUNT is reached (which trigger a repay event) and even though
    // enough fee has been locked.
    //
    // Do another `repay` try during finalization to remedy it.
    if let Err(err) = fee_reserve.repay_all() {
        if invoke_result.is_ok() {
            invoke_result = Err(RuntimeError::ModuleError(ModuleError::CostingError(
                CostingError::FeeReserveError(err),
            )));
        }
    }

    // First - check for required rejections from explicit invoke result errors
    match &invoke_result {
        Err(RuntimeError::ApplicationError(ApplicationError::TransactionProcessorError(err))) => {
            match err {
                TransactionProcessorError::TransactionEpochNotYetValid {
                    valid_from,
                    current_epoch,
                } => {
                    return TransactionResultType::Reject(
                        RejectionError::TransactionEpochNotYetValid {
                            valid_from: *valid_from,
                            current_epoch: *current_epoch,
                        },
                    )
                }
                TransactionProcessorError::TransactionEpochNoLongerValid {
                    valid_until,
                    current_epoch,
                } => {
                    return TransactionResultType::Reject(
                        RejectionError::TransactionEpochNoLongerValid {
                            valid_until: *valid_until,
                            current_epoch: *current_epoch,
                        },
                    )
                }
                _ => {}
            }
        }
        Err(err) => {
            if let Some(abort_reason) = err.abortion() {
                return TransactionResultType::Abort(abort_reason.clone());
            }
        }
        _ => {}
    }

    // Check for errors before loan is repaid - in which case, we also reject
    if !fee_reserve.fully_repaid() {
        return match invoke_result {
            Ok(..) => TransactionResultType::Reject(RejectionError::SuccessButFeeLoanNotRepaid),
            Err(error) => {
                TransactionResultType::Reject(RejectionError::ErrorBeforeFeeLoanRepaid(error))
            }
        };
    }

    TransactionResultType::Commit(invoke_result)
}

fn distribute_fees<S: SubstateDatabase, M: DatabaseMapper>(
    track: &mut Track<S, M>,
    fee_reserve: SystemLoanFeeReserve,
    is_success: bool,
) -> (FeeSummary, IndexMap<NodeId, Decimal>) {
    // Distribute royalty
    for (_, (recipient_vault_id, amount)) in fee_reserve.royalty_cost() {
        let node_id = recipient_vault_id;
        let module_id = SysModuleId::Object;
        let substate_key = FungibleVaultOffset::LiquidFungible.into();
        let (handle, _) = track
            .acquire_lock(
                &node_id,
                module_id.into(),
                &substate_key,
                LockFlags::MUTABLE,
            )
            .unwrap();
        let mut substate: LiquidFungibleResource = track.read_substate(handle).as_typed().unwrap();
        substate.put(LiquidFungibleResource::new(amount)).unwrap();
        track.update_substate(handle, IndexedScryptoValue::from_typed(&substate));
        track.release_lock(handle);
    }

    // Take fee payments
    let fee_summary = fee_reserve.finalize();
    let mut fee_payments: IndexMap<NodeId, Decimal> = index_map_new();
    let mut required = fee_summary.total_execution_cost_xrd + fee_summary.total_royalty_cost_xrd
        - fee_summary.total_bad_debt_xrd;
    for (vault_id, mut locked, contingent) in fee_summary.locked_fees.iter().cloned().rev() {
        let amount = if contingent {
            if is_success {
                Decimal::min(locked.amount(), required)
            } else {
                Decimal::zero()
            }
        } else {
            Decimal::min(locked.amount(), required)
        };

        // Take fees
        locked.take_by_amount(amount).unwrap();
        required -= amount;

        // Refund overpayment
        let (handle, _) = track
            .acquire_lock(
                &vault_id,
                SysModuleId::Object.into(),
                &FungibleVaultOffset::LiquidFungible.into(),
                LockFlags::MUTABLE,
            )
            .unwrap();
        let mut substate: LiquidFungibleResource = track.read_substate(handle).as_typed().unwrap();
        substate.put(locked).unwrap();
        track.update_substate(handle, IndexedScryptoValue::from_typed(&substate));
        track.release_lock(handle);

        // Record final payments
        *fee_payments.entry(vault_id).or_default() += amount;
    }

    // TODO: distribute fees
    (fee_summary, fee_payments)
}
