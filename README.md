# Mavlink2Rest
[![Build status](https://travis-ci.org/patrickelectric/mavlink2rest.svg)](https://travis-ci.org/patrickelectric/mavlink2rest)
[![Cargo download](https://img.shields.io/crates/d/mavlink2rest)](https://crates.io/crates/mavlink2rest)
[![Crate info](https://img.shields.io/crates/v/mavlink2rest.svg)](https://crates.io/crates/mavlink2rest)
[![Documentation](https://docs.rs/mavlink2rest/badge.svg)](https://docs.rs/mavlink2rest)

`mavlink2rest` creates a REST server that provides mavlink information from a mavlink source.

## Install :zap:
- :gear: Cargo Install: `cargo install mavlink2rest`

## Downloads :package:

- :computer: [**Windows**](https://github.com/patrickelectric/mavlink2rest/releases/download/continuous/mavlink2rest-i686-pc-windows-msvc.zip)
- :apple: [**MacOS**](https://github.com/patrickelectric/mavlink2rest/releases/download/continuous/mavlink2rest-x86_64-apple-darwin)
- :penguin: [**Linux**](https://github.com/patrickelectric/mavlink2rest/releases/download/continuous/mavlink2rest-x86_64-unknown-linux-musl)
- :strawberry: [**Raspberry**](https://github.com/patrickelectric/mavlink2rest/releases/download/continuous/mavlink2rest-armv7-unknown-linux-musleabihf)

## Endpoints

### Pages
* Main webpage: `GET /`
  * Provides information about mavlink2rest and available messages.

### API
* MAVLink JSON:
  * `GET /mavlink|/mavlink/*`. The output is a JSON that you get each nested key individually, E.g:
    * http://0.0.0.0:8088/mavlink/ATTITUDE/
    * http://0.0.0.0:8088/mavlink/ATTITUDE/roll
    * http://0.0.0.0:8088/mavlink/ATTITUDE/message_information/time/last_message
      * Any MAVLink message will contain a normal message definition, as described in `GET /helper/message/*`, and a **message_information** structure defined as:
          ```js
          "message_information": {
              "counter": 0, // Number of messages received
              "frequency": 10.0, // Frequency of the received message
              "time": { // ISO 8601 / RFC 3339 date & time format
                  "first_message": "2020-03-28T12:47:52.315383-03:00",
                  "last_message": "2020-03-28T14:16:21.417836-03:00"
              }
          }
          ```
  * `POST /mavlink`. Sends the message to a specific vehicle.
    * For more information about the MAVLink message definition: https://mavlink.io/en/guide/serialization.html
    * **header**: Is the mavlink header definition with `system_id`, `component_id` and `sequence`.
    * **message**: A valid mavlink [message](https://mavlink.io/en/messages/common.html), for more information check `GET /helper/message/*`.
    ```js
    {
        "header": { // MAVLink message header
            "system_id": 1, // System ID
            "component_id": 1, // Component ID
            "sequence": 0 // Message sequence
        },
        "message": { // MAVLink message payload
            "type":"COMMAND_LONG",
            "param1":0.0,
            "param2":0.0,"param3":0.0,"param4":0.0,"param5":0.0,"param6":0.0,"param7":0.0,
            "command":{
            "type":"MAV_CMD_COMPONENT_ARM_DISARM"
            },
            "target_system":0,
            "target_component":0,
            "confirmation":0
        }
    }
    ```
  * `GET /helper/message/MAVLINK_MESSAGE_NAME`: Helper endpoint to create JSON compatible MAVLink messages, where `MAVLINK_MESSAGE_NAME` is the mavlink message name. E.g:
    * http://0.0.0.0:8088/helper/message/COMMAND_LONG
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
                  "type": "MAV_CMD_NAV_WAYPOINT"
              },
              "target_system": 0,
              "target_component": 0,
              "confirmation": 0
          }
      }
      ```

#### Examples

* Get all messages:
  ```sh
  curl --request GET http://0.0.0.0:8088/mavlink\?pretty\=true
  # The output is huge, you can get it here: https://gist.github.com/patrickelectric/26a407c4e7749cdaa58d06b52212cb1e
  ```

* Get attitude:
  ```sh
  curl --request GET http://0.0.0.0:8088/mavlink/ATTITUDE?pretty=true
  ```
  ```js
  {
    "message_information": {
      "counter": 46460,
      "frequency": 7.966392517089844,
      "time": {
        "first_message": "2020-03-28T12:47:52.315383-03:00",
        "last_message": "2020-03-28T14:25:04.905914-03:00"
      }
    },
    "pitch": 0.004207547288388014,
    "pitchspeed": 0.0010630330070853233,
    "roll": 0.004168820567429066,
    "rollspeed": 0.0009180732304230332,
    "time_boot_ms": 6185568,
    "type": "ATTITUDE",
    "yaw": -1.5562472343444824,
    "yawspeed": 0.0009576341835781932
  }
  ````

* Get time of last *ATTITUDE* message
  ```sh
  curl --request GET http://0.0.0.0:8088/mavlink/ATTITUDE/message_information/time/last_message?pretty=true
  ```
  ```js
  "2020-03-28T14:28:51.577853-03:00"
  ```

* Get a message structure example
  ```sh
  curl --request GET http://0.0.0.0:8088/helper/message/ATTITUDE\?pretty\=true
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

* Request vehicle to be [armed](https://mavlink.io/en/messages/common.html#MAV_CMD_COMPONENT_ARM_DISARM)
  ```sh
  # ARM: param1 is 1.0
  curl --request POST http://0.0.0.0:8088/mavlink -H "Content-Type: application/json" --data \
  '{
    "header": {
      "system_id": 1,
      "component_id": 1,
      "sequence": 0
    },
    "message": {
      "type":"COMMAND_LONG",
      "param1":1.0,
      "param2":0.0,"param3":0.0,"param4":0.0,"param5":0.0,"param6":0.0,"param7":0.0,
      "command":{
        "type":"MAV_CMD_COMPONENT_ARM_DISARM"
      },
      "target_system":0,
      "target_component":0,
      "confirmation":0
    }
  }'
  ```

* Request vehicle to be [disarmed](https://mavlink.io/en/messages/common.html#MAV_CMD_COMPONENT_ARM_DISARM)
  ```sh
  # ARM: param1 is 0.0
  curl --request POST http://0.0.0.0:8088/mavlink -H "Content-Type: application/json" --data \
  '{
    "header": {
      "system_id": 1,
      "component_id": 1,
      "sequence": 0
    },
    "message": {
      "type":"COMMAND_LONG",
      "param1":0.0,
      "param2":0.0,"param3":0.0,"param4":0.0,"param5":0.0,"param6":0.0,"param7":0.0,
      "command":{
        "type":"MAV_CMD_COMPONENT_ARM_DISARM"
      },
      "target_system":0,
      "target_component":0,
      "confirmation":0
    }
  }'
  ```

> Note: For any invalid `GET`, you'll receive a 404 response with the error message.
> Note: The endpoints that allow `GET` and provides a JSON output, also allow the usage of the query parameter `pretty` with a boolean value `true` or `false`, E.g: http://0.0.0.0:8088/helper/message/COMMAND_LONG?pretty=true
