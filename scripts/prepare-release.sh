#!/bin/bash

version=$1
merge_master=${2:-}

echo "PREV_VERSION: $PREV_VERSION"
echo "PREV_METADATA: $PREV_METADATA"
echo "NEW_VERSION: $NEW_VERSION"
echo "NEW_METADATA: $NEW_METADATA"
echo "DRY_RUN: $DRY_RUN"
echo "CRATE_NAME: $CRATE_NAME"
echo "WORKSPACE_ROOT: $WORKSPACE_ROOT"
echo "CRATE_ROOT: $CRATE_ROOT"



# git checkout master
# git pull

# if version contains hyphen (meaning either -alpha or -beta release)
# do: git checkout to release-{{version}} branch
# else: git checkout to release-{{version}} tag
# git checkout $(git branch --list | grep -qw 'release-{{version}}' && echo 'release-{{version}}' || echo '-b release-{{version}}')"
