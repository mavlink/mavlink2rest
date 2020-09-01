#!/usr/bin/env python3
import argparse
import asyncio
import aiohttp


async def start_client(url: str) -> None:
    ws = await aiohttp.ClientSession().ws_connect(url, autoclose=False, autoping=False)

    async def dispatch():
        while True:
            msg = await ws.receive()

            if msg.type == aiohttp.WSMsgType.TEXT:
                print("Text: ", msg.data.strip())
            elif msg.type == aiohttp.WSMsgType.BINARY:
                print("Binary: ", msg.data)
            elif msg.type == aiohttp.WSMsgType.PING:
                await ws.pong()
            elif msg.type == aiohttp.WSMsgType.PONG:
                print("Pong received")
            else:
                if msg.type == aiohttp.WSMsgType.CLOSE:
                    await ws.close()
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    print("Error during receive %s" % ws.exception())
                elif msg.type == aiohttp.WSMsgType.CLOSED:
                    pass

                break

    await dispatch()


ARGS = argparse.ArgumentParser(
    description="websocket console client for wssrv.py example."
)
ARGS.add_argument(
    "--url",
    action="store",
    dest="url",
    default="http://0.0.0.0:8088/ws/mavlink?filter=.*",
    help="Websocket address, follow the format: http://0.0.0.0:8088/ws/mavlink?filter={regex}",
)

if __name__ == "__main__":
    args = ARGS.parse_args()

    loop = asyncio.get_event_loop()
    asyncio.Task(start_client(args.url))
    loop.run_forever()
