# Set the current working directory to the root of the repository,
# regardless of where the script is run from
cd "$(git rev-parse --show-toplevel)"

# Build latest version of the plugin
cd ./zellij
RUSTUP_TOOLCHAIN=1.96.1 cargo build --release

# Build the docker image
cd ..

# Pre-flight: ensure all mounted host files exist
wasm=./zellij/target/wasm32-wasip1/release/zellij-tabula.wasm
config=./tests/config.kdl
for f in "$wasm" "$config"; do
  [ -f "$f" ] || { echo "Missing file: $f" >&2; exit 1; }
done

docker build -t zellij:test -f ./tests/Dockerfile . 2>&1 | tail -5


docker run --rm \
  -v "$wasm":/zellij-tabula.wasm:ro \
  -v "$config":/home/alice/.config/zellij/config.kdl:ro \
  zellij:test

status=$?

docker rmi zellij:test 2>/dev/null || true

exit $status