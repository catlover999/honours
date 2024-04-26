# Additive Noise Differential Privacy for Fluent Bit.
##### Using Rust and WASM

Adds differentially private noise to Fluent Bit numeric records based off of dynamically loaded settings files.

This project can be found on Github at: https://github.com/catlover999/honours

## Dependencies
- `podman` or `docker`

## Quickstart guide
1. Extract the tarball, open your terminal interface and navigate to the extracted tarball.
2. Ensure Docker or Podman is installed. If using Docker, ensure your user is either in your sudoers file or otherwise has permission to control the Docker daemon (rootless Docker).
3. Run `podman build --tag=julien-honours .`  
   Or `sudo docker build --tag=julien-honours .`
4. Run `podman run -p 8888:8888 julien-honours`  
   Or `sudo docker run -p 8888:8888 julien-honours`  
   *Note:* Ensure no other running service is running on TCP port 8888. Alter the above command to choose an alternative port mapping if applicable.
5. Navigate to the URL printed in the terminal. You cannot directly connect to localhost:8888 without possessing the access token embedded into the URL.
6. You will be presented with a Jupyter Notebook environment. Navigate to `project.ipynb` where you will find the results evaluated between the input and output datasets.
7. To rerun the notebook, go to "Run">"Run All Cells" in Jupyter's toolbar.

## Repo summary
- Report
    - Report entry point: avdnthesis.tex (must be in repo root)
    - Report content: report/
        - 1 file per chapter
        - misc includes in media/
- rust code: filter_dp/
    - Cargo.toml: dependency list
    - src/lib.rs: main filter plugin code
- Fluent Bit sample configuration:
    - Sample data: input/
    - Sample parser for input: parsers.conf
    - Sample pipeline: fluent-bit.yaml
    - Sample privacy parameters: filters/
- Jupyter Notebook to evaluate perturbed samples: project.ipynb
- Container setup: Dockerfile
- Liscense for use of input datasets, adapted code, and all files written for project: LICENSE

