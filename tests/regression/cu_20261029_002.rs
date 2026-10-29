//! @module cu_20261029_002
//! @description Regression test (tensor-core lock state): the helper's boot-log command is typed, takes no path
//! and returns a summary, never file contents; a lock state is never claimed without a measurement; a half-rate
//! reading after an unlocked boot of this session is a lost unlock; and no check runs with GPU driver telemetry off.
use nitrotray::app::model::AppModel;
use nitrotray::config::settings::Settings;
use nitrotray::ipc::protocol::{encode_line, parse_request, ErrorCode, ProviderStatus, Request, Response};
use nitrotray::service::core::ServiceCore;
use nitrotray::service::simulated::SimulatedBackend;
use nitrotray::service::unlock_log::StaticUnlockLog;
use nitrotray::unlock::boot_log::{parse, BootReport, BootStatus};
use nitrotray::unlock::state::TensorRates;
use nitrotray::unlock::view::ViewState;

const SESSION: u64 = 1_793_000_000;
const HALF_RATE: TensorRates = TensorRates { acc32_tflops: 27.6, acc16_tflops: 55.1 };

#[test]
fn the_boot_log_request_names_no_path() {
    assert_eq!(parse_request(r#"{"op":"get_unlock_boot"}"#), Ok(Request::GetUnlockBoot));
    for smuggled in [r#"{"op":"get_unlock_boot","path":"x"}"#, r#"{"op":"get_unlock_boot","name":"RUN0001.TXT"}"#] {
        assert_eq!(parse_request(smuggled).unwrap_err().code, ErrorCode::InvalidRequest, "{smuggled}");
    }
}

#[test]
fn the_boot_log_answer_is_a_summary_and_never_the_file() {
    let log = "RTX 4060 Laptop unlock build=auto profile=x\nsecret-looking line sha256=abc123\nPOST-GOP SS0=0x88888888 SS1=0x8\n\
TRANSACTION OK\n";
    let report = parse("RUN0028.TXT", log, Some(SESSION));
    let mut core = ServiceCore::new(SimulatedBackend::default(), "m", ProviderStatus::Simulated, None)
        .with_unlock_log(Box::new(StaticUnlockLog(Some(report))));
    let line = encode_line(&core.handle(Request::GetUnlockBoot));
    assert!(!line.contains("secret-looking") && !line.contains("sha256") && !line.contains("POST-GOP"), "{line}");
    let reply: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    let keys: Vec<&str> = reply["unlock_boot"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys.len(), 4, "run, status, build, written_unix and nothing else: {keys:?}");
    assert_eq!(reply["unlock_boot"]["status"], "unlocked");
}

#[test]
fn no_measurement_means_no_claim() {
    let mut m = AppModel::new(Settings::default(), false, false);
    let v = m.ui_state(0).unlock;
    assert_eq!((v.state, v.badge.as_str()), (ViewState::Unknown, ""), "never 'locked' for a GPU nobody measured");
    m.begin_tensor();
    m.on_tensor(Err("nvcuda.dll not found (no NVIDIA driver)".into()), Ok(None), 1_000);
    assert_eq!(m.unlock_view(1_000).state, ViewState::Unknown, "a failed measurement is not a verdict either");
}

#[test]
fn a_half_rate_reading_after_an_unlocked_boot_of_this_session_is_a_lost_unlock() {
    let mut m = AppModel::new(Settings::default(), false, false);
    m.set_session_start(Some(SESSION));
    let boot = BootReport {
        run: "RUN0028".into(),
        status: BootStatus::Unlocked,
        build: "auto".into(),
        written_unix: Some(SESSION - 2),
    };
    m.begin_tensor();
    m.on_tensor(Ok(HALF_RATE), Ok(Some(boot)), 1_000);
    let v = m.unlock_view(1_000);
    assert_eq!((v.state, v.badge.as_str()), (ViewState::Lost, "Unlock lost"));
}

#[test]
fn driver_telemetry_off_forbids_the_check_itself() {
    let mut settings = Settings::default();
    settings.telemetry.gpu_driver_queries = false;
    let m = AppModel::new(settings, false, false);
    assert!(!m.tensor_due(0, false) && !m.tensor_due(0, true), "the probe wakes the GPU like NVML does");
}

#[test]
fn an_older_helper_reply_is_a_plain_error_not_a_crash() {
    let reply = Response::fail(ErrorCode::InvalidRequest, "malformed request: unknown variant");
    assert!(!reply.ok);
    assert_eq!(reply.unlock_boot, None);
}
