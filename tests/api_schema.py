#!/usr/bin/env python3
"""API schema tests for modern (/v1) and legacy (/v0) JSON formats."""

import json
import sys
import time

import requests

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8088"
V1 = f"{BASE}/v1"
V0 = f"{BASE}/v0"


def expect(cond, msg):
    if not cond:
        raise AssertionError(msg)


def post(url, payload):
    response = requests.post(url, data=json.dumps(payload), headers={"Content-Type": "application/json"})
    return response


def main():
    print("info")
    info = requests.get(f"{BASE}/info").json()
    expect(info["service"]["name"] == "mavlink2rest", "bad service name")

    print("helper modern PARAM_SET is string")
    helper_v1 = requests.get(f"{V1}/helper/mavlink", params={"name": "PARAM_SET"}).json()
    expect(isinstance(helper_v1["message"]["param_id"], str), f"v1 param_id={helper_v1['message']['param_id']!r}")

    print("helper legacy PARAM_SET is char array")
    helper_v0 = requests.get(f"{V0}/helper/mavlink", params={"name": "PARAM_SET"}).json()
    expect(isinstance(helper_v0["message"]["param_id"], list), "v0 param_id should be list")
    expect(len(helper_v0["message"]["param_id"]) == 16, "v0 param_id length")

    print("helper modern HEARTBEAT flag string")
    hb_v1 = requests.get(f"{V1}/helper/mavlink", params={"name": "HEARTBEAT"}).json()
    expect(isinstance(hb_v1["message"]["base_mode"], str), "v1 base_mode string")

    print("helper legacy HEARTBEAT bits object")
    hb_v0 = requests.get(f"{V0}/helper/mavlink", params={"name": "HEARTBEAT"}).json()
    expect(isinstance(hb_v0["message"]["base_mode"], dict), "v0 base_mode object")
    expect("bits" in hb_v0["message"]["base_mode"], "v0 base_mode.bits")

    print("POST modern PARAM_SET then GET both schemas")
    modern_param = {
        "header": {"system_id": 1, "component_id": 1, "sequence": 0},
        "message": {
            "type": "PARAM_SET",
            "target_system": 1,
            "target_component": 1,
            "param_id": "SCR_FSS",
            "param_value": 1.25,
            "param_type": {"type": "MAV_PARAM_TYPE_REAL32"},
        },
    }
    response = post(f"{V1}/mavlink", modern_param)
    expect(response.status_code == 200, f"modern post failed: {response.status_code} {response.text}")

    got_v1 = requests.get(f"{V1}/mavlink/vehicles/1/components/1/messages/PARAM_SET").json()
    expect(got_v1["message"]["param_id"] == "SCR_FSS", f"v1 get param_id={got_v1['message']['param_id']!r}")
    expect(got_v1["message"]["param_value"] == 1.25, "v1 param_value")

    got_v0 = requests.get(f"{V0}/mavlink/vehicles/1/components/1/messages/PARAM_SET").json()
    expect(isinstance(got_v0["message"]["param_id"], list), "v0 get param_id list")
    expect("".join(ch for ch in got_v0["message"]["param_id"] if ch != "\u0000") == "SCR_FSS", "v0 joined param_id")

    print("POST legacy PARAM_SET with char array")
    legacy_param = {
        "header": {"system_id": 1, "component_id": 1},
        "message": {
            "type": "PARAM_SET",
            "target_system": 1,
            "target_component": 1,
            "param_id": list("LEGACY_P") + ["\u0000"] * (16 - len("LEGACY_P")),
            "param_value": 3.5,
            "param_type": {"type": "MAV_PARAM_TYPE_REAL32"},
        },
    }
    response = post(f"{V0}/mavlink", legacy_param)
    expect(response.status_code == 200, f"legacy post failed: {response.status_code} {response.text}")
    got = requests.get(f"{V1}/mavlink/vehicles/1/components/1/messages/PARAM_SET").json()
    expect(got["message"]["param_id"] == "LEGACY_P", f"after legacy post got {got['message']['param_id']!r}")

    print("POST legacy HEARTBEAT with bits object")
    legacy_hb = {
        "header": {"system_id": 1, "component_id": 1},
        "message": {
            "type": "HEARTBEAT",
            "custom_mode": 0,
            "mavtype": {"type": "MAV_TYPE_GENERIC"},
            "autopilot": {"type": "MAV_AUTOPILOT_GENERIC"},
            "base_mode": {"bits": 129},
            "system_status": {"type": "MAV_STATE_ACTIVE"},
            "mavlink_version": 3,
        },
    }
    response = post(f"{V0}/mavlink", legacy_hb)
    expect(response.status_code == 200, f"legacy hb post failed: {response.text}")
    got = requests.get(f"{V1}/mavlink/vehicles/1/components/1/messages/HEARTBEAT").json()
    mode = got["message"]["base_mode"]
    expect(isinstance(mode, str), "modern get base_mode string")
    expect("MAV_MODE_FLAG_SAFETY_ARMED" in mode, mode)
    expect("MAV_MODE_FLAG_CUSTOM_MODE_ENABLED" in mode, mode)
    got_legacy = requests.get(f"{V0}/mavlink/vehicles/1/components/1/messages/HEARTBEAT").json()
    expect(got_legacy["message"]["base_mode"] == {"bits": 129}, got_legacy["message"]["base_mode"])

    print("JSON pointer leaf under both schemas")
    leaf_v1 = requests.get(f"{V1}/mavlink/vehicles/1/components/1/messages/HEARTBEAT/message/base_mode")
    expect(leaf_v1.status_code == 200, leaf_v1.text)
    expect("SAFETY_ARMED" in leaf_v1.text, leaf_v1.text)
    leaf_v0 = requests.get(f"{V0}/mavlink/vehicles/1/components/1/messages/HEARTBEAT/message/base_mode")
    expect(leaf_v0.status_code == 200, leaf_v0.text)
    expect('"bits"' in leaf_v0.text, leaf_v0.text)

    print("malformed legacy bits rejected")
    bad = {
        "header": {"system_id": 1, "component_id": 1},
        "message": {
            "type": "HEARTBEAT",
            "custom_mode": 0,
            "mavtype": {"type": "MAV_TYPE_GENERIC"},
            "autopilot": {"type": "MAV_AUTOPILOT_GENERIC"},
            "base_mode": {"bits": 0x100},
            "system_status": {"type": "MAV_STATE_ACTIVE"},
            "mavlink_version": 3,
        },
    }
    response = post(f"{V0}/mavlink", bad)
    expect(response.status_code == 404, f"expected 404 got {response.status_code}")
    expect(
        ("unsupported bits" in response.text)
        or ("Failed to parse" in response.text)
        or ("invalid type" in response.text),
        response.text,
    )

    print("malformed modern JSON rejected")
    response = post(f"{V1}/mavlink", {"header": {}, "message": {"type": "NOPE"}})
    expect(response.status_code == 404, response.text)

    print("ordinary ATTITUDE helper both schemas match shape")
    att_v1 = requests.get(f"{V1}/helper/mavlink", params={"name": "ATTITUDE"}).json()
    att_v0 = requests.get(f"{V0}/helper/mavlink", params={"name": "ATTITUDE"}).json()
    expect(att_v1["message"]["type"] == "ATTITUDE", "v1 attitude")
    expect(att_v0["message"]["type"] == "ATTITUDE", "v0 attitude")
    expect(att_v1["message"]["roll"] == att_v0["message"]["roll"], "roll")

    print("websocket modern and legacy")
    import asyncio
    import aiohttp

    async def ws_check(url, expect_bits_object):
        session = aiohttp.ClientSession()
        ws = await session.ws_connect(url)
        # seed a heartbeat through REST so WS clients see something if quiet
        post(f"{V1}/mavlink", {
            "header": {"system_id": 1, "component_id": 1},
            "message": {
                "type": "NAMED_VALUE_FLOAT",
                "time_boot_ms": 1,
                "name": list("WSTEST") + ["\u0000"] * (10 - len("WSTEST")) if False else "WSTEST",
                "value": 42.0,
            },
        })
        # For modern name is string; post modern:
        post(f"{V1}/mavlink", {
            "header": {"system_id": 1, "component_id": 1},
            "message": {
                "type": "NAMED_VALUE_FLOAT",
                "time_boot_ms": 1,
                "name": "WSTEST",
                "value": 42.0,
            },
        })
        msg = None
        for _ in range(20):
            incoming = await asyncio.wait_for(ws.receive(), timeout=2)
            if incoming.type == aiohttp.WSMsgType.TEXT:
                payload = json.loads(incoming.data)
                if payload.get("message", {}).get("type") == "NAMED_VALUE_FLOAT":
                    msg = payload
                    break
        await ws.close()
        await session.close()
        expect(msg is not None, f"no NAMED_VALUE_FLOAT on {url}")
        name = msg["message"]["name"]
        if expect_bits_object:
            expect(isinstance(name, list), f"legacy name should be list, got {name!r}")
        else:
            expect(isinstance(name, str), f"modern name should be str, got {name!r}")

    async def run_ws_checks():
        await ws_check(f"{V1}/ws/mavlink?filter=NAMED_VALUE_FLOAT", False)
        await ws_check(f"{V0}/ws/mavlink?filter=NAMED_VALUE_FLOAT", True)

    asyncio.run(run_ws_checks())

    print("OK")


if __name__ == "__main__":
    main()
