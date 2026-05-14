#!/bin/bash
set -euo pipefail

# This script tars the repo and pushes it up to Azure to be built and pushed to ACR (Azure Container Registry).
#
# We separate this script from the `justfile` to eliminate dependencies so a CI pipeline can push without a dependency
# on `just`. We also don't have a dependency on `docker`, since it's built remotely. Azure CLI and `git` is all that's
# needed.
dir="$(dirname "$(realpath "$0")")"
az acr build \
    --image "ai-driven-website/bundle:${GITHUB_SHA:-"local-build"}" \
    --registry genericregistry "${dir}"
