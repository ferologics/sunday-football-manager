#!/usr/bin/env bash
# Extract changelog section for a version
# Usage: ./scripts/changelog.sh 0.3.0
VERSION=$1
awk "/^## ${VERSION}/{flag=1; next} /^## /{flag=0} flag" CHANGELOG.md
