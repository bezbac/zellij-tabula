# Set the current working directory to the root of the repository,
# regardless of where the script is run from
cd "$(git rev-parse --show-toplevel)"

# Build latest version of the plugin
cd ./zellij
cargo build --release

# Build the docker image
cd ..
docker build -t zellij:test -f ./tests/Dockerfile .

# Run the expect script inside the docker container
docker run --rm -v ./tests/__snapshots__:/tests/__snapshots__ -t zellij:test

# Delete the docker image after the test is done
docker rmi zellij:test
