# Cargo commands

Run simulation

```bash
    cargo run --bin new_sim
```

Build docs
```bash
    cargo doc --open
```

Test code
```bash
    cargo test
```

Profile code
```bash
   cargo run --bin new_sim --profile=release
   samply record C:/temp/rust-builds/md/release/md.exe
```
View results in browser

## My own commands

In the .bin folder we have various bash scripts which act as a shorthand for various commands.

Remove all contents of the folder.
```bash
    cleanup folder_name
```

Copies the folder and contents to a new location named "folder_name" + date and time stamp, then deletes contents in the original folder.
```bash
    keep folder_name
```
Run a python script with name "script_name.py" in the "python_scripts" folder.
```bash
    setup script_name
```

Run runs a simulation. If the optional flag c is provided it will cleanup any existing files under that script_name in output. If the optional flag -r is provided it will build everything in the optimised release mode. It then runs the appropriate python script before building and running the simulation.
```bash
    run -c script_name
```

Video will run the script video.rs which takes all the .parquet data files and uses them to create a video.
```bash
    video script_name
```

Benchmarking with the Samply add on. We have setup a load of particles falling into a silo.
```bash
    benchmark
```

