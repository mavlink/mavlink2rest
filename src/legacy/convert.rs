use mavlink::Message as ModernMessage;
use mavlink_legacy::Message as LegacyMessage;
use serde_json::Value;

type ModernMavMessage = mavlink::dialects::ardupilotmega::MavMessage;
type LegacyApmMessage = mavlink_legacy::ardupilotmega::MavMessage;
type LegacyCommonMessage = mavlink_legacy::common::MavMessage;

pub fn modern_to_legacy(value: &Value) -> Result<Value, String> {
    let mut out = value.clone();
    convert_tree(&mut out, Direction::ModernToLegacy)?;
    Ok(out)
}

pub fn legacy_to_modern(value: &Value) -> Result<Value, String> {
    let mut out = value.clone();
    convert_tree(&mut out, Direction::LegacyToModern)?;
    Ok(out)
}

#[derive(Clone, Copy)]
enum Direction {
    ModernToLegacy,
    LegacyToModern,
}

fn convert_tree(value: &mut Value, direction: Direction) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if let (Some(header), Some(message)) =
                (map.get("header").cloned(), map.get("message").cloned())
            {
                if message.get("type").and_then(Value::as_str).is_some() {
                    let converted = match direction {
                        Direction::ModernToLegacy => modern_message_value_to_legacy(message)?,
                        Direction::LegacyToModern => legacy_message_value_to_modern(message)?,
                    };
                    map.insert("header".into(), header);
                    map.insert("message".into(), converted);
                    return Ok(());
                }
            }
            if let Some(Value::String(type_name)) = map.get("type").cloned() {
                if looks_like_message_body(map) {
                    let body = Value::Object(map.clone());
                    let converted = match direction {
                        Direction::ModernToLegacy => modern_message_value_to_legacy(body)?,
                        Direction::LegacyToModern => legacy_message_value_to_modern(body)?,
                    };
                    *value = converted;
                    return Ok(());
                }
                let _ = type_name;
            }
            for child in map.values_mut() {
                convert_tree(child, direction)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                convert_tree(item, direction)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn looks_like_message_body(map: &serde_json::Map<String, Value>) -> bool {
    map.contains_key("type") && map.len() > 1 && !map.contains_key("header")
}

fn modern_message_value_to_legacy(message: Value) -> Result<Value, String> {
    let modern: ModernMavMessage = serde_json::from_value(message)
        .map_err(|error| format!("Failed to parse modern message for legacy convert: {error}"))?;
    modern_message_to_legacy_value(&modern)
}

fn modern_message_to_legacy_value(modern: &ModernMavMessage) -> Result<Value, String> {
    let mut payload = [0u8; 255];
    let size = ModernMessage::ser(modern, mavlink::MavlinkVersion::V2, &mut payload);
    let legacy = <LegacyApmMessage as LegacyMessage>::parse(
        mavlink_legacy::MavlinkVersion::V2,
        modern.message_id(),
        &payload[..size],
    )
    .map_err(|error| {
        format!(
            "Legacy dialect cannot represent {}: {error:?}",
            modern.message_name()
        )
    })?;
    serialize_legacy_message(&legacy)
}

fn serialize_legacy_message(message: &LegacyApmMessage) -> Result<Value, String> {
    match message {
        LegacyApmMessage::common(inner) => serde_json::to_value(inner)
            .map_err(|error| format!("Failed to serialize legacy common message: {error}")),
        other => serde_json::to_value(other)
            .map_err(|error| format!("Failed to serialize legacy message: {error}")),
    }
}

fn legacy_message_value_to_modern(message: Value) -> Result<Value, String> {
    let legacy = parse_legacy_message(message)?;
    let payload = LegacyMessage::ser(&legacy);
    let modern = <ModernMavMessage as ModernMessage>::parse(
        mavlink::MavlinkVersion::V2,
        LegacyMessage::message_id(&legacy),
        &payload,
    )
    .map_err(|error| {
        format!(
            "Modern dialect cannot represent {}: {error:?}",
            LegacyMessage::message_name(&legacy)
        )
    })?;
    serde_json::to_value(modern)
        .map_err(|error| format!("Failed to serialize modern message: {error}"))
}

fn parse_legacy_message(message: Value) -> Result<LegacyApmMessage, String> {
    match serde_json::from_value::<LegacyCommonMessage>(message.clone()) {
        Ok(common) => Ok(LegacyApmMessage::common(common)),
        Err(common_error) => match serde_json::from_value::<LegacyApmMessage>(message) {
            Ok(message) => Ok(message),
            Err(apm_error) => Err(format!(
                "Failed to parse legacy MAVLink message: common={common_error}; apm={apm_error}"
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn param_set_roundtrip_via_wire() {
        let modern = json!({
            "type": "PARAM_SET",
            "target_system": 1,
            "target_component": 1,
            "param_id": "SCR_FSS",
            "param_value": 1.5,
            "param_type": {"type": "MAV_PARAM_TYPE_REAL32"}
        });
        let legacy = modern_to_legacy(&modern).unwrap();
        assert!(legacy.get("param_id").unwrap().is_array(), "{legacy:?}");
        let back = legacy_to_modern(&legacy).unwrap();
        assert_eq!(back.get("param_id").unwrap(), "SCR_FSS");
    }

    #[test]
    fn heartbeat_flags_roundtrip_via_wire() {
        let modern = json!({
            "type": "HEARTBEAT",
            "custom_mode": 0,
            "mavtype": {"type": "MAV_TYPE_GENERIC"},
            "autopilot": {"type": "MAV_AUTOPILOT_GENERIC"},
            "base_mode": "MAV_MODE_FLAG_SAFETY_ARMED | MAV_MODE_FLAG_CUSTOM_MODE_ENABLED",
            "system_status": {"type": "MAV_STATE_UNINIT"},
            "mavlink_version": 3
        });
        let legacy = modern_to_legacy(&modern).unwrap();
        assert_eq!(legacy["base_mode"], json!({"bits": 129}));
        let back = legacy_to_modern(&legacy).unwrap();
        let mode = back.get("base_mode").unwrap().as_str().unwrap();
        assert!(mode.contains("MAV_MODE_FLAG_SAFETY_ARMED"));
    }
}
