#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn red_phase_scaffold_compiles_fails_tests_warns_clippy_and_preserves_manual_tests() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap();
    let chromatofoor_path = repo_root.join("chromatofoor");
    let shared_target_dir = repo_root.join("target/red-phase-test");

    let tmp = tempfile::tempdir().unwrap();
    let spec_dir = tmp.path().join("spec");
    let out_base = tmp.path().join("out");
    fs::create_dir_all(&spec_dir).unwrap();

    let model_yaml = r"apiVersion: schema.esdm.io/core/v1
kind: domain
name: red-phase-demo
description: Minimal domain verifying Red-phase scaffold guarantees.
---
apiVersion: schema.esdm.io/core/v1
kind: bounded-context
name: catalog
description: Catalog bounded context.
scope:
  domain: red-phase-demo
---
apiVersion: schema.esdm.io/core/v1
kind: actor
name: operator
description: Catalog operator.
type: human
scope:
  domain: red-phase-demo
  boundedContext: catalog
---
apiVersion: schema.esdm.io/core/v1
kind: aggregate
name: item
description: Catalog item aggregate.
scope:
  domain: red-phase-demo
  boundedContext: catalog
identifiedBy:
  source: generated
  generator: uuid
state:
  properties:
    title:
      type: string
  required: [title]
---
apiVersion: schema.esdm.io/core/v1
kind: command
name: register-item
description: Registers a catalog item.
scope:
  domain: red-phase-demo
  boundedContext: catalog
  aggregate: item
actors: [operator]
data:
  properties:
    title:
      type: string
  required: [title]
publishes: [item-registered]
---
apiVersion: schema.esdm.io/core/v1
kind: event
name: item-registered
description: Emitted when a catalog item is registered.
scope:
  domain: red-phase-demo
  boundedContext: catalog
  aggregate: item
data:
  properties:
    title:
      type: string
  required: [title]
---
apiVersion: schema.esdm.io/core/v1
kind: read-model
name: item-summary
description: Read model summarizing the latest registered item.
scope:
  domain: red-phase-demo
  boundedContext: catalog
projections:
  - boundedContext: catalog
    aggregate: item
    event: item-registered
schema:
  properties:
    title:
      type: string
  required: [title]
---
apiVersion: schema.esdm.io/core/v1
kind: query
name: get-item-summary
description: Fetches the latest item summary.
scope:
  domain: red-phase-demo
  boundedContext: catalog
readModel: item-summary
actors: [operator]
result:
  type: object
";

    let lifecycle_yaml = r"apiVersion: schema.esdm.io/lifecycle/v1
kind: aggregate-lifecycle
name: item-lifecycle
description: Lifecycle rules for catalog item.
scope:
  domain: red-phase-demo
  boundedContext: catalog
  aggregate: item
created: [register-item]
";

    let infra_yaml = r"apiVersion: schema.esdm.io/infrastructure/v1
kind: presentation-binding
name: catalog-cli
description: CLI presentation binding to keep test workspace minimal.
scope:
  domain: red-phase-demo
  boundedContext: catalog
transport: cli
cli:
  command: catalog
";

    let feature_yaml_full = r"apiVersion: schema.esdm.io/given-when-then/v1
kind: feature
name: item-registration
description: Item registration command behavior.
scope:
  domain: red-phase-demo
  boundedContext: catalog
  aggregate: item
scenarios:
  - name: idempotent-noop
    description: Expecting zero events still fails in Red phase due to todo stub.
    given: []
    when:
      actor: operator
      command: register-item
      data:
        title: Widget
    then:
      events: []
---
apiVersion: schema.esdm.io/given-when-then/v1
kind: feature
name: item-summary-query
description: Item summary query behavior.
scope:
  domain: red-phase-demo
  boundedContext: catalog
  readModel: item-summary
scenarios:
  - name: returns-expected-title
    description: Empty history query asserting then.result fails in Red phase.
    given: []
    when:
      actor: operator
      query: get-item-summary
      parameters: {}
    then:
      result:
        title: Widget
";

    fs::write(spec_dir.join("model.esdm.yaml"), model_yaml).unwrap();
    fs::write(spec_dir.join("model.lifecycle.esdm.yaml"), lifecycle_yaml).unwrap();
    fs::write(spec_dir.join("model.infra.esdm.yaml"), infra_yaml).unwrap();
    fs::write(spec_dir.join("model.feature.esdm.yaml"), feature_yaml_full).unwrap();

    let gen_bin = env!("CARGO_BIN_EXE_chromatofoor-gen");
    let gen_out = Command::new(gen_bin)
        .arg("--input")
        .arg(spec_dir.join("model.esdm.yaml"))
        .arg("--out-dir")
        .arg(&out_base)
        .arg("--chromatofoor-path")
        .arg(chromatofoor_path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(
        gen_out.status.success(),
        "chromatofoor-gen failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&gen_out.stdout),
        String::from_utf8_lossy(&gen_out.stderr)
    );

    let ws_dir = out_base.join("red-phase-demo");
    let bc_dir = ws_dir.join("bounded-contexts/catalog");

    // 1. Verify workspace and member lint inheritance
    let root_cargo = fs::read_to_string(ws_dir.join("Cargo.toml")).unwrap();
    assert!(root_cargo.contains("unsafe_code = \"deny\""));
    assert!(root_cargo.contains("todo = \"warn\""));
    let bc_cargo = fs::read_to_string(bc_dir.join("Cargo.toml")).unwrap();
    assert!(bc_cargo.contains("[lints]\nworkspace = true"));

    // Add a manual integration test alongside tests/generated/
    let manual_test_path = bc_dir.join("tests/custom_manual.rs");
    fs::write(
        &manual_test_path,
        "#[test]\nfn manual_test_survives() {\n    assert_eq!(2 + 2, 4);\n}\n",
    )
    .unwrap();

    // 2. Verify workspace compiles cleanly and passes clippy when clippy::todo is allowed
    let no_run = Command::new("cargo")
        .args(["test", "--workspace", "--no-run"])
        .current_dir(&ws_dir)
        .env("CARGO_TARGET_DIR", &shared_target_dir)
        .output()
        .unwrap();
    assert!(
        no_run.status.success(),
        "cargo test --no-run failed:\n{}",
        String::from_utf8_lossy(&no_run.stderr)
    );

    let clippy_allow_todo = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
            "-A",
            "clippy::todo",
        ])
        .current_dir(&ws_dir)
        .env("CARGO_TARGET_DIR", &shared_target_dir)
        .output()
        .unwrap();
    assert!(
        clippy_allow_todo.status.success(),
        "cargo clippy -A clippy::todo failed:\n{}",
        String::from_utf8_lossy(&clippy_allow_todo.stderr)
    );

    // 3. Verify generated Red-phase tests fail deterministically while manual test passes
    let test_run = Command::new("cargo")
        .args(["test", "-p", "catalog"])
        .current_dir(&ws_dir)
        .env("CARGO_TARGET_DIR", &shared_target_dir)
        .output()
        .unwrap();
    assert!(
        !test_run.status.success(),
        "expected Red-phase generated tests to fail, but cargo test succeeded"
    );
    let test_stdout = String::from_utf8_lossy(&test_run.stdout);
    let test_stderr = String::from_utf8_lossy(&test_run.stderr);
    assert!(
        test_stdout.contains("manual_test_survives ... ok"),
        "expected manual_test_survives to pass:\nstdout:\n{test_stdout}\nstderr:\n{test_stderr}"
    );
    assert!(
        test_stdout.contains("aggregate_item::test_item_registration_idempotent_noop ... FAILED"),
        "missing aggregate scenario failure in stdout:\nstdout:\n{test_stdout}\nstderr:\n{test_stderr}"
    );
    assert!(
        test_stdout.contains(
            "read_model_item_summary::test_item_summary_query_returns_expected_title ... FAILED"
        ),
        "missing read-model scenario failure in stdout:\nstdout:\n{test_stdout}\nstderr:\n{test_stderr}"
    );

    // 4. Verify cargo clippy -D clippy::todo fails on unimplemented stubs
    let clippy_deny_todo = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "clippy::todo",
        ])
        .current_dir(&ws_dir)
        .env("CARGO_TARGET_DIR", &shared_target_dir)
        .output()
        .unwrap();
    assert!(
        !clippy_deny_todo.status.success(),
        "expected cargo clippy -D clippy::todo to fail on unimplemented stubs"
    );

    // 5. Remove read-model feature and regenerate -> verify stale purge & manual preservation
    let feature_yaml_agg_only = r"apiVersion: schema.esdm.io/given-when-then/v1
kind: feature
name: item-registration
description: Item registration command behavior.
scope:
  domain: red-phase-demo
  boundedContext: catalog
  aggregate: item
scenarios:
  - name: idempotent-noop
    description: Expecting zero events still fails in Red phase due to todo stub.
    given: []
    when:
      actor: operator
      command: register-item
      data:
        title: Widget
    then:
      events: []
";
    fs::write(
        spec_dir.join("model.feature.esdm.yaml"),
        feature_yaml_agg_only,
    )
    .unwrap();

    let regen_out = Command::new(gen_bin)
        .arg("--input")
        .arg(spec_dir.join("model.esdm.yaml"))
        .arg("--out-dir")
        .arg(&out_base)
        .arg("--chromatofoor-path")
        .arg(chromatofoor_path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(regen_out.status.success());

    assert!(
        !bc_dir
            .join("tests/generated/read_model_item_summary.rs")
            .exists(),
        "stale read_model_item_summary.rs should have been purged"
    );
    assert!(bc_dir.join("tests/generated/aggregate_item.rs").is_file());
    assert!(manual_test_path.is_file());

    // 6. Verify fail-closed feature parsing does not wipe existing generated tests
    fs::write(
        spec_dir.join("model.feature.esdm.yaml"),
        "apiVersion: schema.esdm.io/given-when-then/v1\nkind: [invalid_yaml\n",
    )
    .unwrap();
    let bad_out = Command::new(gen_bin)
        .arg("--input")
        .arg(spec_dir.join("model.esdm.yaml"))
        .arg("--out-dir")
        .arg(&out_base)
        .arg("--chromatofoor-path")
        .arg(chromatofoor_path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(!bad_out.status.success());
    assert!(
        bc_dir.join("tests/generated/aggregate_item.rs").is_file(),
        "fail-closed abort must preserve existing generated tests"
    );
}
