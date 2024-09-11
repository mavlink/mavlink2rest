# Mavlink2Rest
[![Deploy mavlink2rest](https://github.com/mavlink/mavlink2rest/actions/workflows/deploy.yml/badge.svg)](https://github.com/mavlink/mavlink2rest/actions/workflows/deploy.yml)
[![Test](https://github.com/mavlink/mavlink2rest/actions/workflows/test.yml/badge.svg)](https://github.com/mavlink/mavlink2rest/actions/workflows/test.yml)
[![Cargo download](https://img.shields.io/crates/d/mavlink2rest)](https://crates.io/crates/mavlink2rest)
[![Crate info](https://img.shields.io/crates/v/mavlink2rest.svg)](https://crates.io/crates/mavlink2rest)
[![Documentation](https://docs.rs/mavlink2rest/badge.svg)](https://docs.rs/mavlink2rest)

`mavlink2rest` is a tool that offers a RESTful API over the MAVLink protocol, facilitating seamless communication between unmanned systems and web applications. The tool supports the ArduPilotMega dialect, iCAROUS, and UAVionix, making it an ideal solution for developers who want to build custom interfaces for unmanned systems.

The current version supports the **ardupilotmega** dialect, that includes **common**, **icarous** and **uavionix**.

## Grab it
### Downloads :package:

[Latest builds](https://github.com/mavlink/mavlink2rest/releases/latest):
- :computer: [**Windows**](https://github.com/mavlink/mavlink2rest/releases/latest/download/mavlink2rest-x86_64-pc-windows-msvc.exe)
- :apple: [**MacOS**](https://github.com/mavlink/mavlink2rest/releases/latest/download/mavlink2rest-x86_64-apple-darwin)
- :penguin: [**Linux**](https://github.com/mavlink/mavlink2rest/releases/latest/download/mavlink2rest-x86_64-unknown-linux-musl)
- :strawberry: [**Raspberry**](https://github.com/mavlink/mavlink2rest/releases/latest/download/mavlink2rest-arm-unknown-linux-musleabihf)
  - [ARMv6 binary](https://github.com/mavlink/mavlink2rest/releases/latest/download/mavlink2rest-arm-unknown-linux-musleabihf), [ARMv7](https://github.com/mavlink/mavlink2rest/releases/latest/download/mavlink2rest-armv7-unknown-linux-musleabihf) is also available under the project releases.

For others or different releases, check the [releases menu](https://github.com/mavlink/mavlink2rest/releases).

### Install :zap:
If you prefer, you can install via cargo, if you don't know what it is, use the [download section](https://github.com/mavlink/mavlink2rest#downloads-package).
- :gear: Cargo Install: `cargo install mavlink2rest`

## Help
Capabilities via the command line:
```
USAGE:
  mavlink2rest [FLAGS] [OPTIONS]

FLAGS:
  -h, --help       Prints help information
  -V, --version    Prints version information
  -v, --verbose    Be verbose

OPTIONS:
      --component-id <COMPONENT_ID>
          Sets the component ID for this service, for more information, check:
          https://mavlink.io/en/messages/common.html#MAV_COMPONENT [default: 0]
  -c, --connect <TYPE:<IP/SERIAL>:<PORT/BAUDRATE>>
          Sets the mavlink connection string [default: udpin:0.0.0.0:14550]

      --mavlink <VERSION>
          Sets the mavlink version used to communicate [default: 2]  [possible values: 1, 2]

  -s, --server <IP:PORT>
          Sets the IP and port that the rest server will be provided [default: 0.0.0.0:8088]

      --system-id <SYSTEM_ID>                Sets system ID for this service. [default: 255]
```
# Using Docker with mavlink2rest

You can also use the `mavlink2rest` with docker, the following command will start the service with the default settings:
```sh
docker run --rm --init -p 8088:8088 -p 14550:14550/udp --name mavlink2rest mavlink/mavlink2rest
```

The Dockerfile defines several environment variables that you can override at runtime:

MAVLINK_SRC: The MAVLink source connection string. Default is udpin:127.0.0.1:14550.
SERVER_PORT: The IP and port for the REST server. Default is 0.0.0.0:8088.
EXTRA_ARGS: Any additional command line arguments you want to pass to mavlink2rest.
To customize these settings, use the -e flag with docker run:

```sh
docker run --rm --init\
    -p 8088:8088 \
    -p 14551:14551/udp \
    -e MAVLINK_SRC="udpin:0.0.0.0:14551" \
    -e SERVER_PORT="0.0.0.0:8088" \
    --name mavlink2rest mavlink/mavlink2rest
```

to build the docker image locally, you can use the following command:
```sh
docker build --build-arg TARGET_ARCH=x86_64-unknown-linux-musl -t mavlink/mavlink2rest .
```

## Endpoints

### Pages
* Main webpage: `GET /`
  * Provides information about mavlink2rest and available messages.
* Swagger: `GET /docs`
  * Provides information about mavlink2rest endpoints for the REST API.

### API
* MAVLink JSON:
  * `GET /v1/mavlink|/v1/mavlink/*`. The output is a JSON that you get each nested key individually, E.g:
    * http://0.0.0.0:8088/v1/mavlink/vehicles/1/components/1/messages/ATTITUDE
    * http://0.0.0.0:8088/v1/mavlink/vehicles/1/components/1/messages/ATTITUDE/roll
    * http://0.0.0.0:8088/v1/mavlink/vehicles/1/components/1/messages/ATTITUDE/status/time/last_update
  * Any MAVLink message will contain a normal message definition, as described in `GET /v1/helper/mavlink?name=<MESSAGE_NAME>`..
    * http://0.0.0.0:8088/v1/helper/mavlink?name=HEARTBEAT
      ```js
      {
        "header": {
          "system_id": 255,
          "component_id": 0,
          "sequence": 0
        },
        "message": {
          "type": "HEARTBEAT",
          "custom_mode": 0,
          "mavtype": {
            "type": "MAV_TYPE_GENERIC"
          },
          "autopilot": {
            "type": "MAV_AUTOPILOT_GENERIC"
          },
          "base_mode": {
            "bits": 128
          },
          "system_status": {
            "type": "MAV_STATE_UNINIT"
          },
          "mavlink_version": 0
        }
      }
      ```
      > :sunglasses: This is really hand when creating messages. 
  * ... and a **status** structure defined as:
    ```js
    "status": {
      "time": {
        "counter": 2981,
        "first_update": "2024-05-31T11:59:44.313941926-03:00",
        "frequency": 10.037036895751953,
        "last_update": "2024-05-31T12:04:42.214212884-03:00"
      }
    }
    ```
  * `POST /mavlink`. Sends the message to a specific vehicle.
    * For more information about the MAVLink message definition: https://mavlink.io/en/guide/serialization.html
    * **header**: Is the mavlink header definition with `system_id`, `component_id` and `sequence`.
    * **message**: A valid mavlink [message](https://mavlink.io/en/messages/common.html), for more information check `GET /v1/helper/mavlink?name=<MESSAGE_NAME>`.
      * Check [ARM/DISARM example](https://github.com/mavlink/mavlink2rest#examples).

  * `GET /v1/helper/mavlink?name=MAVLINK_MESSAGE_NAME`: Helper endpoint to create JSON compatible MAVLink messages, where `MAVLINK_MESSAGE_NAME` is the mavlink message name. E.g:
    * http://0.0.0.0:8088/v1/helper/mavlink?name=COMMAND_LONG
      ```js
      {
          "header": {
              "system_id": 255,
              "component_id": 0,
              "sequence": 0
          },
          "message": {
              "type": "COMMAND_LONG",
              "param1": 0.0,
              "param2": 0.0,
              "param3": 0.0,
              "param4": 0.0,
              "param5": 0.0,
              "param6": 0.0,
              "param7": 0.0,
              "command": {
                  "type": "MAV_CMD_NAV_WAYPOINT" // Random value
              },
              "target_system": 0,
              "target_component": 0,
              "confirmation": 0
          }
      }
      ```
* Information:
  * `GET /info`, provides information about the service version.
    * http://0.0.0.0:8088/info
      ```js
      {
        "version": 0,
        "service": {
          "name": "mavlink2rest",
          "version": "0.10.0",
          "sha": "bd7667d",
          "build_date": "2021-03-03",
          "authors": "Author <email>"
        }
      }
      ```


#### Examples

##### Get all messages:
  ```sh
  curl --request GET http://0.0.0.0:8088/v1/mavlink
  # The output is huge, you can get it here: [https://gist.github.com/patrickelectric/26a407c4e7749cdaa58d06b52212cb1e](https://gist.github.com/patrickelectric/26a407c4e7749cdaa58d06b52212cb1e)
  ```

##### Get attitude:
  ```sh
  curl --request GET http://0.0.0.0:8088/v1/mavlink/vehicles/1/components/1/messages/ATTITUDE
  ```
  ```js
  {
    "message": {
      "pitch": 0.11506611853837967,
      "pitchspeed": 0.00003909762017428875,
      "roll": 0.02339238114655018,
      "rollspeed": 0.00035849903360940516,
      "time_boot_ms": 87110407,
      "type": "ATTITUDE",
      "yaw": -2.4364013671875,
      "yawspeed": 0.000020137056708335876
    },
    "status": {
      "time": {
        "counter": 22750,
        "first_update": "2024-05-31T11:59:44.313941926-03:00",
        "frequency": 5.495169162750244,
        "last_update": "2024-05-31T13:08:45.196118069-03:00"
      }
    }
  }
  ````

##### Get time of last *ATTITUDE* message:
  ```sh
  curl --request GET http://0.0.0.0:8088/v1/mavlink/ATTITUDE/status/time/last_update
  ```
  ```js
  "2020-03-28T14:28:51.577853-03:00"
  ```

##### Get a message structure example:
  ```sh
  curl --request GET http://0.0.0.0:8088/v1/helper/mavlink?name=ATTITUDE
  ```
  ```js
  {
    "header": {
      "system_id": 255,
      "component_id": 0,
      "sequence": 0
    },
    "message": {
      "type": "ATTITUDE",
      "time_boot_ms": 0,
      "roll": 0.0,
      "pitch": 0.0,
      "yaw": 0.0,
      "rollspeed": 0.0,
      "pitchspeed": 0.0,
      "yawspeed": 0.0
    }
  }
  ```

##### Request vehicle to be [armed](https://mavlink.io/en/messages/common.html#MAV_CMD_COMPONENT_ARM_DISARM):
  ```sh
  # ARM: param1 is 1.0
  curl --request POST http://0.0.0.0:8088/v1/mavlink -H "Content-Type: application/json" --data \
  '{
    "header": {
      "system_id": 255,
      "component_id": 240,
      "sequence": 0
    },
    "message": {
      "type":"COMMAND_LONG",
      "param1": 1.0,
      "param2": 0.0,"param3":0.0,"param4":0.0,"param5":0.0,"param6":0.0,"param7":0.0,
      "command": {
        "type": "MAV_CMD_COMPONENT_ARM_DISARM"
      },
      "target_system": 1,
      "target_component": 1,
      "confirmation": 1
    }
  }'
  ```

##### Request vehicle to be [disarmed](https://mavlink.io/en/messages/common.html#MAV_CMD_COMPONENT_ARM_DISARM):
  ```sh
  # ARM: param1 is 0.0
  curl --request POST http://0.0.0.0:8088/v1/mavlink -H "Content-Type: application/json" --data \
  '{
    "header": {
      "system_id": 255,
      "component_id": 240,
      "sequence": 0
    },
    "message": {
      "type":"COMMAND_LONG",
      "param1": 0.0,
      "param2": 0.0,"param3":0.0,"param4":0.0,"param5":0.0,"param6":0.0,"param7":0.0,
      "command": {
        "type": "MAV_CMD_COMPONENT_ARM_DISARM"
      },
      "target_system": 1,
      "target_component": 1,
      "confirmation": 1
    }
  }'
  ```

> Note: For any invalid `GET`, you'll receive a 404 response with the error message.
> Note: The endpoints that allow `GET` and provides a JSON output, also allow the usage of the query parameter `pretty` with a boolean value `true` or `false`, E.g: http://0.0.0.0:8088/helper/mavlink?name=COMMAND_LONG&pretty=true

### Websocket

It's also possible to connect multiple websockets with the following path `/ws/mavlink`, the endpoint also accepts the query parameter `filter`, the filter value should be a regex that matches MAVLink message names, E.g: `/ws/mavlink?filter=.*` for all messages, `/ws/mavlink?filter=RC_.*` will match **RC_CHANNELS_RAW** and **RC_CHANNELS**, resulting in the following output:
  ```json
  { // First message
    "header": {
      "component_id": 1,
      "sequence": 98,
      "system_id": 1
    },
    "message": {
      "chan10_raw": 0,
      "chan11_raw": 0,
      "chan12_raw": 0,
      "chan13_raw": 0,
      "chan14_raw": 0,
      "chan15_raw": 0,
      "chan16_raw": 0,
      "chan17_raw": 0,
      "chan18_raw": 0,
      "chan1_raw": 1500,
      "chan2_raw": 1500,
      "chan3_raw": 1500,
      "chan4_raw": 1500,
      "chan5_raw": 1500,
      "chan6_raw": 1500,
      "chan7_raw": 1500,
      "chan8_raw": 1500,
      "chan9_raw": 0,
      "chancount": 16,
      "message_information": {
        "counter": 3732,
        "frequency": 4.0,
        "time": {
          "first_message": "2020-09-01T20:36:24.088099-03:00",
          "last_message": "2020-09-01T20:51:57.278901-03:00"
        }
      },
      "rssi": 0,
      "time_boot_ms": 3122812,
      "type": "RC_CHANNELS"
    }
  }
  { // Second message
    "header": {
      "component_id": 1,
      "sequence": 98,
      "system_id": 1
    },
    "message": {
      "chan1_raw": 1500,
      "chan2_raw": 1500,
      "chan3_raw": 1500,
      "chan4_raw": 1500,
      "chan5_raw": 1500,
      "chan6_raw": 1500,
      "chan7_raw": 1500,
      "chan8_raw": 1500,
      "message_information": {
        "counter": 3732,
        "frequency": 4.0,
        "time": {
          "first_message": "2020-09-01T20:36:24.088310-03:00",
          "last_message": "2020-09-01T20:51:57.279438-03:00"
        }
      },
      "port": 0,
      "rssi": 0,
      "time_boot_ms": 3122812,
      "type": "RC_CHANNELS_RAW"
    }
  }
  ```
For a demonstration, please check the example under the examples filder: `websocket_client.py`

# Benchmark
The following benchmarks were extracted from a raspberry pi 3 connected to a pixhawk running ArduSub.
- In idle.
    ```
    6% CPU usage
    ```
- 1 client requesting all mavlink messages at 10Hz
    ```
    9% CPU usage
    ```
- 1 client requesting all mavlink messages at 100Hz
    ```
    20% CPU usage (~5% each core)
    ```
- 1 websocket with no filters
    ```
    11% CPU usage
    ```
- 5 websockets with no filters
    ```
    24% CPU usage (14% @ 1 core, ~3% @ 3 cores)
    ```
- 20 websockets with filter only for **ATTITUDE** message (receiving at 10Hz)
    ```
    9% CPU usage
    ```
- 20 websockets with filter only for **NAMED_VALUE_FLOAT** message (receiving at 70Hz)
    ```
    17% CPU usage (9% @ 1 core, ~2% @ 3 cores)
    ```
- 20 websockets with no filters
    ```
    48% CPU usage (20% @ 1 core, ~9% @ 3 cores)
    ```
- 1 client requesting all mavlink messages 1000 times
    ```
    60% CPU usage (~15% each core)
    Time taken for tests      3.7 seconds
    Total requests            1000
    Successful requests       1000
    Failed requests           0
    Requests per second       273.60 [#/sec]
    Median time per request   3ms
    Average time per request  4ms
    ```

- 10 clients requesting all mavlink messages, 100 requests for each client.
    ```
    140% CPU usage (~46% each core)
    Time taken for tests      1.4 seconds
    Total requests            1000
    Successful requests       1000
    Failed requests           0
    Requests per second       733.14 [#/sec]
    Median time per request   13ms
    Average time per request  13ms
    Sample standard deviation 3ms
    ```

- 100 clients requesting all mavlink messages, 1000 requests for each client.
    ```
    140% CPU usage (~46% each core)
    Time taken for tests      13.8 seconds
    Total requests            10000
    Successful requests       10000
    Failed requests           0
    Requests per second       725.83 [#/sec]
    Median time per request   132ms
    Average time per request  137ms
    Sample standard deviation 54ms
    ```
