# Set the current working directory to the root of the repository,
# regardless of where the script is run from
cd "$(git rev-parse --show-toplevel)"

# Build latest version of the plugin
cd ./zellij
cargo build --release

# Build the docker image
cd ..

# Pre-flight: ensure all mounted host files exist
wasm=./zellij/target/wasm32-wasip1/release/zellij-tabula.wasm
config=./tests/config.kdl
for f in "$wasm" "$config" ./tests/package.json; do
  [ -f "$f" ] || { echo "Missing file: $f" >&2; exit 1; }
done

docker build -t zellij:test -f ./tests/Dockerfile . 2>&1 | tail -1

# Run each test file in its own container to give each a fresh zellij daemon.
# The CwdChanged event has a known multi-session interaction issue in zellij
# v0.44.x where panes from prior sessions corrupt tracking for later sessions.
failed=0
for test_file in tests/*.test.js; do
  test_name=$(basename "$test_file")
  echo "--- $test_name ---"
  if docker run --rm -t \
    -v "$wasm":/zellij-tabula.wasm:ro \
    -v "$config":/home/alice/.config/zellij/config.kdl:ro \
    -v ./tests:/tests \
    -v /tests/node_modules \
    --entrypoint ./node_modules/.bin/tui-test \
    zellij:test \
    "$test_file"; then
    echo "PASS: $test_name"
  else
    echo "FAIL: $test_name"
    failed=$((failed + 1))
  fi
done

docker rmi zellij:test 2>/dev/null || true

if [ "$failed" -gt 0 ]; then
  echo "$failed test(s) failed"
  exit 1
fi
echo "All tests passed"
