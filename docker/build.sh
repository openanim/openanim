#!/usr/bin/env bash
# Build all OpenAnim renderer Docker images.
# Run from the docker/ directory: ./build.sh [--push]
set -euo pipefail

PUSH=${1:-}
IMAGES=(manim mermaid ffmpeg remotion)

for img in "${IMAGES[@]}"; do
    echo "━━━ Building openanim/${img}:latest ━━━"
    docker build -f "Dockerfile.${img}" -t "openanim/${img}:latest" .
    if [[ "$PUSH" == "--push" ]]; then
        docker push "openanim/${img}:latest"
    fi
done

echo ""
echo "✓ All images built."
echo "  To use Docker mode in the engine:"
echo "    SandboxConfig cfg;"
echo "    cfg.mode = SandboxMode::Docker;"
echo "    cfg.docker_image = \"openanim/<provider>:latest\";"
echo "    OpenAnimEngine engine(\".openanim/cache\", cfg);"
