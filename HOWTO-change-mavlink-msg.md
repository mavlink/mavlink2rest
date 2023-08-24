### Updating MAVLink Message Set

To update the MAVLink message set, you will need to rebuild both mavlink2rest and [rust-mavlink](https://github.com/mavlink/rust-mavlink). As of August 2023, this process is not straightforward and requires following specific instructions.

Firstly, you need to set up a working RUST environment. Follow the official instructions at https://www.rust-lang.org/ to set up your Rust environment.

Next, obtain the source code of rust-mavlink and update its MAVLink submodule to acquire the correct MAVLink revision:
```sh
git clone git@github.com:mavlink/rust-mavlink.git --recursive
```

Retrieve the source code of mavlink2rest as well:
```sh
git clone https://github.com/khancyr/mavlink2rest --recursive
```

Select the version of mavlink2rest you desire. Open the `Cargo.toml` file to determine the corresponding version of rust-mavlink. Then, checkout the desired version of rust-mavlink, for example:
```sh
cd rust-mavlink/
git checkout 0.10.2
```

Now, you can edit the MAVLink message definitions located in `mavlink/message_definitions/v1.0`.

Compile rust-mavlink with the desired message set you wish to support. In this example, we'll use the default `ardupilotmega.xml`:
```sh
cargo install --path . --features="ardupilotmega emit-extensions"
```

Once the compilation is finished, return to the `Cargo.toml` file of mavlink2rest and update the path to the local rust-mavlink library. 

Before:
```toml
mavlink = { git = "https://github.com/mavlink/rust-mavlink", rev = "0.10.2", features = [ "ardupilotmega", "emit-extensions"] }
```

After:
```toml
mavlink = { path="/home/khancyr/Workspace/rust-mavlink", features = [ "ardupilotmega", "emit-extensions"] }
```

This change points to the updated rust-mavlink library containing the updated MAVLink messages.

Finally, compile mavlink2rest:
```sh
cargo install --path .
```

Your newly compiled mavlink2rest will now comprehend your newly added messages.

