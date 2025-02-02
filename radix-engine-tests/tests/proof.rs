use radix_engine::errors::{RuntimeError, SystemModuleError};
use radix_engine::system::system_modules::auth::AuthError;
use radix_engine::system::system_modules::node_move::NodeMoveError;
use radix_engine::types::*;
use radix_engine_interface::blueprints::resource::FromPublicKey;
use scrypto::resource::DIVISIBILITY_MAXIMUM;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;
use utils::ContextualDisplay;

#[test]
fn can_create_clone_and_drop_bucket_proof() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address = test_runner.create_non_fungible_resource(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_from_account(account, resource_address, 1.into())
        .take_all_from_worktop(resource_address, |builder, bucket_id| {
            builder.call_function(
                package_address,
                "BucketProof",
                "create_clone_drop_bucket_proof",
                manifest_args!(bucket_id, dec!("1")),
            )
        })
        .call_method(
            account,
            "try_deposit_batch_or_abort",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );
    println!(
        "{}",
        receipt.display(&AddressBech32Encoder::for_simulator())
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_create_clone_and_drop_vault_proof() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address = test_runner.create_non_fungible_resource(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, resource_address, 1.into())
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "create_clone_drop_vault_proof",
            manifest_args!(Decimal::one()),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    println!(
        "{}",
        receipt.display(&AddressBech32Encoder::for_simulator())
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_create_clone_and_drop_vault_proof_by_amount() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address =
        test_runner.create_fungible_resource(100.into(), DIVISIBILITY_MAXIMUM, account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, resource_address, 3.into())
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "create_clone_drop_vault_proof_by_amount",
            manifest_args!(dec!("3"), dec!("1")),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    println!(
        "{}",
        receipt.display(&AddressBech32Encoder::for_simulator())
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_create_clone_and_drop_vault_proof_by_ids() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address = test_runner.create_non_fungible_resource(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, resource_address, 3.into())
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let non_fungible_local_ids = BTreeSet::from([
        NonFungibleLocalId::integer(1),
        NonFungibleLocalId::integer(2),
        NonFungibleLocalId::integer(3),
    ]);
    let proof_non_fungible_local_ids = BTreeSet::from([NonFungibleLocalId::integer(2)]);
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "create_clone_drop_vault_proof_by_ids",
            manifest_args!(non_fungible_local_ids, proof_non_fungible_local_ids),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_use_bucket_for_authorization() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let (auth_resource_address, burnable_resource_address) =
        test_runner.create_restricted_burn_token(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_from_account(account, auth_resource_address, 1.into())
        .withdraw_from_account(account, burnable_resource_address, 1.into())
        .take_all_from_worktop(auth_resource_address, |builder, auth_bucket_id| {
            builder.take_all_from_worktop(
                burnable_resource_address,
                |builder, burnable_bucket_id| {
                    builder.call_function(
                        package_address,
                        "BucketProof",
                        "use_bucket_proof_for_auth",
                        manifest_args!(auth_bucket_id, burnable_bucket_id),
                    )
                },
            )
        })
        .call_method(
            account,
            "try_deposit_batch_or_abort",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_use_vault_for_authorization() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let (auth_resource_address, burnable_resource_address) =
        test_runner.create_restricted_burn_token(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, auth_resource_address, 1.into())
                .take_all_from_worktop(auth_resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_from_account(account, burnable_resource_address, 1.into())
        .take_all_from_worktop(burnable_resource_address, |builder, bucket_id| {
            builder.call_method(
                component_address,
                "use_vault_proof_for_auth",
                manifest_args!(bucket_id),
            )
        })
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_create_proof_from_account_and_pass_on() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address =
        test_runner.create_fungible_resource(100.into(), DIVISIBILITY_MAXIMUM, account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .create_proof_from_account_of_amount(account, resource_address, 1.into())
        .pop_from_auth_zone(|builder, proof_id| {
            builder.call_function(
                package_address,
                "VaultProof",
                "receive_proof",
                manifest_args!(proof_id),
            )
        })
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn cant_move_restricted_proof() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address =
        test_runner.create_fungible_resource(100u32.into(), DIVISIBILITY_MAXIMUM, account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .create_proof_from_account_of_amount(account, resource_address, 1.into())
        .pop_from_auth_zone(|builder, proof_id| {
            builder.call_function(
                package_address,
                "VaultProof",
                "receive_proof_and_push_to_auth_zone",
                manifest_args!(proof_id),
            )
        })
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::NodeMoveError(
                NodeMoveError::CantMoveDownstream(..)
            ))
        )
    });
}

#[test]
fn can_move_restricted_proofs_internally() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let (public_key, _, account) = test_runner.new_allocated_account();
    let component_address = {
        let manifest = ManifestBuilder::new()
            .call_function(package_address, "Outer", "instantiate", manifest_args!())
            .build();
        let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![]);
        receipt.expect_commit_success().new_component_addresses()[0]
    };

    // Act
    let manifest = ManifestBuilder::new()
        .create_proof_from_account(account, RADIX_TOKEN)
        .create_proof_from_auth_zone(RADIX_TOKEN, |builder, proof| {
            builder.call_method(
                component_address,
                "pass_fungible_proof",
                manifest_args!(proof),
            )
        })
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_move_locked_bucket() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address =
        test_runner.create_fungible_resource(100u32.into(), DIVISIBILITY_MAXIMUM, account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_from_account(account, resource_address, 1.into())
        .take_all_from_worktop(resource_address, |builder, bucket_id| {
            builder.call_function(
                package_address,
                "BucketProof",
                "return_bucket_while_locked",
                manifest_args!(bucket_id),
            )
        })
        .call_method(
            account,
            "try_deposit_batch_or_abort",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_compose_bucket_and_vault_proof() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address =
        test_runner.create_fungible_resource(100u32.into(), DIVISIBILITY_MAXIMUM, account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, resource_address, 1.into())
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_from_account(account, resource_address, 99u32.into())
        .take_from_worktop(resource_address, 99u32.into(), |builder, bucket_id| {
            builder.call_method(
                component_address,
                "compose_vault_and_bucket_proof",
                manifest_args!(bucket_id),
            )
        })
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_compose_bucket_and_vault_proof_by_amount() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address =
        test_runner.create_fungible_resource(100u32.into(), DIVISIBILITY_MAXIMUM, account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, resource_address, 1.into())
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_from_account(account, resource_address, 99u32.into())
        .take_from_worktop(resource_address, 99u32.into(), |builder, bucket_id| {
            builder.call_method(
                component_address,
                "compose_vault_and_bucket_proof_by_amount",
                manifest_args!(bucket_id, Decimal::from(2u32)),
            )
        })
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_compose_bucket_and_vault_proof_by_ids() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address = test_runner.create_non_fungible_resource(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_non_fungibles_from_account(
                    account,
                    resource_address,
                    &btreeset!(NonFungibleLocalId::integer(1)),
                )
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .withdraw_non_fungibles_from_account(
            account,
            resource_address,
            &BTreeSet::from([
                NonFungibleLocalId::integer(2),
                NonFungibleLocalId::integer(3),
            ]),
        )
        .take_non_fungibles_from_worktop(
            resource_address,
            &BTreeSet::from([
                NonFungibleLocalId::integer(2),
                NonFungibleLocalId::integer(3),
            ]),
            |builder, bucket_id| {
                builder.call_method(
                    component_address,
                    "compose_vault_and_bucket_proof_by_ids",
                    manifest_args!(
                        bucket_id,
                        BTreeSet::from([
                            NonFungibleLocalId::integer(1),
                            NonFungibleLocalId::integer(2),
                        ])
                    ),
                )
            },
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_create_vault_proof_by_amount_from_non_fungibles() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address = test_runner.create_non_fungible_resource(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(
        btreeset![NonFungibleGlobalId::from_public_key(&public_key)],
        |builder| {
            builder
                .withdraw_from_account(account, resource_address, 3.into())
                .take_all_from_worktop(resource_address, |builder, bucket_id| {
                    builder.call_function(
                        package_address,
                        "VaultProof",
                        "new",
                        manifest_args!(bucket_id),
                    )
                })
        },
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "create_clone_drop_vault_proof_by_amount",
            manifest_args!(Decimal::from(3), Decimal::from(1)),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_create_auth_zone_proof_by_amount_from_non_fungibles() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (public_key, _, account) = test_runner.new_allocated_account();
    let resource_address = test_runner.create_non_fungible_resource(account);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .create_proof_from_account_of_non_fungibles(
            account,
            resource_address,
            &BTreeSet::from([
                NonFungibleLocalId::integer(1),
                NonFungibleLocalId::integer(2),
            ]),
        )
        .create_proof_from_account_of_non_fungibles(
            account,
            resource_address,
            &BTreeSet::from([NonFungibleLocalId::integer(3)]),
        )
        .create_proof_from_auth_zone_of_non_fungibles(
            resource_address,
            &BTreeSet::from([
                NonFungibleLocalId::integer(2),
                NonFungibleLocalId::integer(3),
            ]),
            |builder, proof_id| {
                builder.call_function(
                    package_address,
                    "Receiver",
                    "assert_ids",
                    manifest_args!(
                        proof_id,
                        BTreeSet::from([
                            NonFungibleLocalId::integer(2),
                            NonFungibleLocalId::integer(3)
                        ]),
                        resource_address
                    ),
                )
            },
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn can_not_call_vault_lock_fungible_amount_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(btreeset![], |builder| {
        builder.call_function(
            package_address,
            "VaultLockUnlockAuth",
            "new_fungible",
            manifest_args!(),
        )
    });

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "call_lock_fungible_amount_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_vault_unlock_fungible_amount_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(btreeset![], |builder| {
        builder.call_function(
            package_address,
            "VaultLockUnlockAuth",
            "new_fungible",
            manifest_args!(),
        )
    });

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "call_lock_fungible_amount_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_vault_lock_non_fungibles_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(btreeset![], |builder| {
        builder.call_function(
            package_address,
            "VaultLockUnlockAuth",
            "new_non_fungible",
            manifest_args!(),
        )
    });

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "call_lock_non_fungibles_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_vault_unlock_non_fungibles_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");
    let component_address = test_runner.new_component(btreeset![], |builder| {
        builder.call_function(
            package_address,
            "VaultLockUnlockAuth",
            "new_non_fungible",
            manifest_args!(),
        )
    });

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_method(
            component_address,
            "call_lock_non_fungibles_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_bucket_lock_fungible_amount_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_function(
            package_address,
            "BucketLockUnlockAuth",
            "call_lock_fungible_amount_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_bucket_unlock_fungible_amount_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_function(
            package_address,
            "BucketLockUnlockAuth",
            "call_lock_fungible_amount_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_bucket_lock_non_fungibles_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_function(
            package_address,
            "BucketLockUnlockAuth",
            "call_lock_non_fungibles_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}

#[test]
fn can_not_call_bucket_unlock_non_fungibles_directly() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/proof");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_function(
            package_address,
            "BucketLockUnlockAuth",
            "call_lock_non_fungibles_directly",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
            _,
        ))) => true,
        _ => false,
    })
}
