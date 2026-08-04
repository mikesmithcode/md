## Adding new models

To create a new simulation:

1. In the input folder create a config file called `new_model.json`:
    - If the model will need new params then you'll need to add these with a new model type

    ```json
        "model": {
            "type": "Active",
            "stiffness": 100000.0,
            "damping": 300.0,
            "v0": 1.0,
            "Dt": 1.0
        }
    ```

2. Initialisation scripts. In the python_scripts folder create a python script called `new_model.py` which defines the initial positions, velocities etc.

3. In the src/bin folder Create a rust script called `new_model.rs`. Start with the `template.rs` and modify. You'll need to implement the functions in the traits Motion and Forces. 

4. If you added new model in your config to read in additional params this will need to be added to be added to the `md::md_sim::models`. Update the enum and either inline or as a new struct unpack the new params. 
