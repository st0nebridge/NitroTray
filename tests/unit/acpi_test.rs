//! @module acpi_test
//! @description AcerGamingFunction bit layouts, checked against real AN515-58 probe outputs (2026-09-30).
use nitrotray::acer::acpi::*;

#[test]
fn decodes_supported_sensors_from_probe() {
    let set = decode_supported_sensors(0x2_2700_0000).unwrap();
    assert_eq!(set.0, 0x227);
    for s in Sensor::ALL {
        assert!(set.contains(s), "{s:?}");
    }
    let only_cpu = SensorSet(0b1);
    assert!(only_cpu.contains(Sensor::CpuTemperature));
    assert!(!only_cpu.contains(Sensor::GpuTemperature));
    assert_eq!(supported_sensors_input(), 0);
}

#[test]
fn builds_sensor_reading_commands() {
    assert_eq!(sensor_reading_input(Sensor::CpuTemperature), 0x0101);
    assert_eq!(sensor_reading_input(Sensor::CpuFanSpeed), 0x0201);
    assert_eq!(sensor_reading_input(Sensor::ExternalTemperature2), 0x0301);
    assert_eq!(sensor_reading_input(Sensor::GpuFanSpeed), 0x0601);
    assert_eq!(sensor_reading_input(Sensor::GpuTemperature), 0x0A01);
}

#[test]
fn decodes_sensor_readings_from_probe() {
    assert_eq!(decode_sensor_reading(0x4100), Ok(65));
    assert_eq!(decode_sensor_reading(0x16_8900), Ok(5769));
    assert_eq!(decode_sensor_reading(0x17_EA00), Ok(6122));
    assert_eq!(decode_sensor_reading(0x3100), Ok(49));
    assert_eq!(decode_sensor_reading(0xFF_FF00), Ok(0xFFFF));
    assert_eq!(decode_sensor_reading(0x1_0000_4100), Ok(65), "bits above 23 ignored");
}

#[test]
fn non_zero_status_is_an_error() {
    assert_eq!(decode_sensor_reading(0x4101), Err(FirmwareStatus(1)));
    assert_eq!(decode_supported_sensors(0x03), Err(FirmwareStatus(3)));
    assert_eq!(decode_fan_speed(0x3202), Err(FirmwareStatus(2)));
    assert_eq!(decode_misc(0x04FF), Err(FirmwareStatus(0xFF)));
    assert_eq!(decode_fan_behavior(0x4101, FAN_BITS_BOTH), Err(FirmwareStatus(1)));
    assert_eq!(decode_set_status(0), Ok(()));
    assert_eq!(decode_set_status(0x0102), Err(FirmwareStatus(2)));
}

#[test]
fn fan_behaviour_layout() {
    assert_eq!(fan_behavior_get_input(FAN_BITS_BOTH), 0x09);
    let both = decode_fan_behavior(0x4100, FAN_BITS_BOTH).unwrap();
    assert_eq!((both.cpu, both.gpu), (Some(1), Some(1)));
    let turbo = decode_fan_behavior((2 << 8) | (2 << 14), FAN_BITS_BOTH).unwrap();
    assert_eq!((turbo.cpu, turbo.gpu), (Some(2), Some(2)));
    let cpu_only = decode_fan_behavior(0x4300, FAN_BIT_CPU).unwrap();
    assert_eq!((cpu_only.cpu, cpu_only.gpu), (Some(3), None));
    let gpu_only = decode_fan_behavior(0xC000, FAN_BIT_GPU).unwrap();
    assert_eq!((gpu_only.cpu, gpu_only.gpu), (None, Some(3)));
}

#[test]
fn fan_behaviour_set_layout() {
    assert_eq!(fan_behavior_set_input(FAN_BITS_BOTH, 1), 0x0040_0009 | (1 << 16));
    assert_eq!(fan_behavior_set_input(FAN_BITS_BOTH, 2), 0x0009 | (2 << 16) | (2 << 22));
    assert_eq!(fan_behavior_set_input(FAN_BITS_BOTH, 3), 0x0009 | (3 << 16) | (3 << 22));
    assert_eq!(fan_behavior_set_input(FAN_BIT_CPU, 2), 0x0001 | (2 << 16));
    assert_eq!(fan_behavior_set_input(FAN_BIT_GPU, 2), 0x0008 | (2 << 22));
    assert_eq!(fan_behavior_set_input(FAN_BITS_BOTH, 0x7), 0x0009 | (3 << 16) | (3 << 22), "mode masked to 2 bits");
}

#[test]
fn fan_speed_layout() {
    assert_eq!(fan_speed_get_input(FanId::Cpu), 0x01);
    assert_eq!(fan_speed_get_input(FanId::Gpu), 0x04);
    assert_eq!(decode_fan_speed(0x3200), Ok(50));
    assert_eq!(fan_speed_set_input(FanId::Cpu, 75), Some(0x4B01));
    assert_eq!(fan_speed_set_input(FanId::Gpu, 100), Some(0x6404));
    assert_eq!(fan_speed_set_input(FanId::Gpu, 0), Some(0x0004));
    assert_eq!(fan_speed_set_input(FanId::Cpu, 101), None);
}

#[test]
fn misc_setting_layout_and_profiles() {
    assert_eq!(misc_get_input(MISC_PLATFORM_PROFILE), 0x0B);
    assert_eq!(decode_misc(0x0400), Ok(4));
    assert_eq!(decode_misc(0x3300), Ok(0x33));
    assert_eq!(misc_set_input(MISC_PLATFORM_PROFILE, 0x04), 0x040B);
    assert_eq!(misc_set_input(MISC_PLATFORM_PROFILE, 0x00), 0x000B);
    let set = ProfileSet(0x33);
    for p in [0, 1, 4, 5] {
        assert!(set.contains(p), "profile {p}");
    }
    for p in [2, 3, 6, 7, 8, 200] {
        assert!(!set.contains(p), "profile {p}");
    }
}

#[test]
fn method_names_match_the_wmi_class() {
    assert_eq!(METHOD_GET_SYS_INFO, "GetGamingSysInfo");
    assert_eq!(METHOD_SET_FAN_BEHAVIOR, "SetGamingFanBehavior");
    assert_eq!(METHOD_GET_FAN_BEHAVIOR, "GetGamingFanBehavior");
    assert_eq!(METHOD_SET_FAN_SPEED, "SetGamingFanSpeed");
    assert_eq!(METHOD_GET_FAN_SPEED, "GetGamingFanSpeed");
    assert_eq!(METHOD_SET_MISC_SETTING, "SetGamingMiscSetting");
    assert_eq!(METHOD_GET_MISC_SETTING, "GetGamingMiscSetting");
    assert_eq!(Sensor::GpuTemperature.id(), 0x0A);
}
