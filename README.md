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
  * `POST /mavlink`. Sends the message to a specific vehicle.
    * For more information about the MAVLink message definition: https://mavlink.io/en/guide/serialization.html
    * **header**: Is the mavlink header definition with `system_id`, `component_id` and `sequence`.
    * **message**: A valid mavlink [message](https://mavlink.io/en/messages/common.html), for more information check `GET /helper/message/*`.
    ```json
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
      ```json
      {
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
      ```

> Note: For any invalid `GET`, you'll receive a 404 response with the error message.
> Note: The endpoints that allow `GET` and provides a JSON output, also allow the usage of the query parameter `pretty` with a boolean value `true` or `false`, E.g: http://0.0.0.0:8088/helper/message/COMMAND_LONG?pretty=true
