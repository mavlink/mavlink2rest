#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["requests", "websockets"]
# ///

import asyncio
import json

import requests
import websockets

API = "http://0.0.0.0:8088/v1"
# DEBUG is never sent by the vehicle, so only our messages go through the filter
WS = "ws://0.0.0.0:8088/v1/ws/mavlink?filter=DEBUG"

message = {
    "header": {"system_id": 255, "component_id": 240, "sequence": 0},
    "message": {"type": "DEBUG", "time_boot_ms": 0, "value": 42.0, "ind": 1},
}


async def receive(listener):
    try:
        received = json.loads(await asyncio.wait_for(listener.recv(), 5))
    except asyncio.TimeoutError:
        return None
    return received["message"]["type"], received["header"]["component_id"]


async def main():
    async with websockets.connect(WS) as listener, websockets.connect(WS) as sender:
        print("Test REST forward..")
        assert requests.post(f"{API}/mavlink", json=message).ok, "Failed to post message."
        assert await receive(listener) == ("DEBUG", 240), "REST message was not forwarded."

        print("Test websocket forward..")
        await sender.send(json.dumps(message))
        assert await receive(listener) == ("DEBUG", 240), "Websocket message was not forwarded."


asyncio.run(main())
