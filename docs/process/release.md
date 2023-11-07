This document describes the release process for `dfx`, with step-by-step
instructions, information about automation, and a checklist.

# Overview {#_overview}

Our first step is to ensure the proper and valid state of the `master`
branch. Next, we update `cargo` and the manifest accordingly. We then
create and push an annotated tag on the `stable` branch, generate the
changelog. The product and SDK team members can then inspect, clarify,
and develop the changelog to ensure it is appropriate for public
consumption. After ensuring the proper artifacts are available in Github
Releases, we can now publish them by updating the manifest.

Before starting the release process, the team should conduct a brief
Go/No-Go release review to evaluate the current state of fixes and
features ready to be included in a release candidate. After reviewing
the list of fixes and features, the team will decide whether to proceed
with staging a build.

If the new release is given the Go green light, two people who are
familiar with the process---a **driver** and a **validator**---use the
steps in this document to stage or promote a release candidate.

A successful release is the result of coordination between automation,
manual steps performed by team members, and a validation process.

## Participants {#_participants}

-   Driver / Primary - the person executing steps listed in this
    document

-   Validator / Secondary - the person most familiar with the process
    and be able to assist with debugging or resolving issues if the
    **driver** building the release runs into trouble.

## Prerequisites {#_prerequisites}

For now, our release process is driven by the DFINITY foundation SDK
team. Future work needs to be done to separate the build and release
process to run fully on open systems.

-   As a **driver**, you must have a GitHub account and push permission
    to the `dfinity/sdk` repository. If you're DFINITY employee and run
    into any issues accessing repositories, you can contact IT using the
    `#help-it` Slack channel.

## Preliminary validation {#_preliminary_validation}

Verify the general stability of the master branch before attempting to
create a release candidate.

1.  Use this [link](https://github.com/dfinity/sdk/commits/master) to
    verify:

    a.  Is `master` green?

    b.  Was `master` red recently or flaky?

        ![is master green](is-master-green.png)

## Preparation {#_preparation}

1.  Open a terminal and `cd` into your local copy of the `sdk` repo.

## Creating a New Release Branch {#_creating_a_new_release_branch}

1.  Check out the `master` branch and pull merged commits from the
    remote in your working directory by running the following command:

    ``` bash
    git checkout master && git pull
    ```

2.  Create the release branch. Note that the branch name never includes
    alpha, beta, and so forth. All of these will be released on the same
    release branch.

    ``` bash
    git switch -c release-<n.n.n> && git push
    ```

    For example, if creating the release branch for 0.7.0, you would run
    the following command:

    ``` bash
    git switch -c release-0.7.0 && git push
    ```

3.  Edit CHANGELOG.md to remove the \"UNRELEASED\" note from the version
    to be released. Commit this to change to the release branch.

## Resuming on an Existing Release Branch {#_resuming_on_an_existing_release_branch}

1.  Check out the `master` branch and pull merged commits from the
    remote in your working directory by running the following command:

    ``` bash
    git checkout release-<n.n.n> && git pull
    ```

## Ready {#_ready}

At this point, you are ready to build a release candidate. There are two
ways you can build a release candidate:

-   Using the [SCRIPT-BASED release process](#script) to automate the
    build process and skip manual testing.

-   Using the [MANUAL STEPS release process](#manual) to build and test
    the release.

# SCRIPT-BASED release process {#script}

To use the release script to automate building the release candidate:

1.  Run the following command and substitute `<n.n.n>` with the version
    number for this release candidate:

    ``` bash
    ./scripts/release.sh <n.n.n>
    ```

    For example, if releasing 0.7.0, you would run the following
    command:

        ./scripts/release.sh 0.7.0

2.  Follow the prompts displayed to complete the release process.

After running this command to build the release candidate, follow the
steps in [Notification and post-build validations](#post-build) to
complete the release process.

# MANUAL STEPS release process {#manual}

The manual release process provides full instructions for building and
testing the release candidate binaries to ensure everything is working
before making a release available to internal or external developers.

## Build DFX {#_build_dfx}

Verify that you can build DFX from the `master` branch without errors.

1.  Build the `dfx` binary by running the following command:

    ``` bash
    cargo clean --release
    cargo build --release --locked
    export dfx_rc="$(pwd)/target/release/dfx"
    ```

    Wait for the `cargo build` to complete (it can take a while). These
    commands create the binary then stores the binary in a shell
    variable.

2.  Make sure the `$dfx_rc` variable points to a real file by running
    the following command:

        test -x $dfx_rc \
            && echo 'Please proceed.' \
            || echo 'Cant find executable $dfx_rc'="$dfx_rc"

    You should see \'Please proceed\' returned.

3.  Delete the existing `dfx` cache to ensure you're not using a stale
    binary.

    ``` bash
    $dfx_rc cache delete
    ```

4.  Ensure `dfx` and `replica` are not running in the background by
    running the following command:

    ``` bash
    ps -ef | grep -E 'replica|dfx' | grep -v grep
    ```

    If there are any `replica` or `dfx` processes running, use the
    `kill` command to terminate them.

## Validate the default project {#_validate_the_default_project}

Verify that you can build, deploy, and call the default `hello_world`
project without errors.

1.  Generate a default new project and change to the project directory
    by running the following commands:

    ``` bash
    $dfx_rc new hello_world
    cd hello_world
    ```

2.  Start the local `replica` as a background process by running the
    following command:

    ``` bash
    $dfx_rc start --clean --background
    ```

3.  Create, build, and install canisters by running the following
    command:

    ``` bash
    $dfx_rc deploy
    ```

4.  Call the canister and verify the result by running the following
    command:

    ``` bash
    $dfx_rc canister call hello_world greet everyone
    ```

5.  Save the canister URLs as shell variables, then print them by
    running the following commands:

    ``` bash
    export hello_world_backend_candid_url="http://localhost:4943/candid?canisterId=$($dfx_rc canister id hello_world_backend)"
    export hello_world_frontend_url="http://localhost:4943/?canisterId=$($dfx_rc canister id hello_world_frontend)"
    ```

6.  Open a web browser and clear your cache or switch to Private
    Browsing/Incognito mode.

7.  Open the following URL in your web browser:

    ``` bash
    echo "$hello_world_frontend_url"
    ```

8.  Verify that you are prompted to type a greeting in a prompt window.

    a.  Type a greeting, then click **OK** to return the greeting in an
        alert window.

    b.  Verify there are no errors in the console by opening the
        Developer Tools.

        For example, in the browser, right-click, then click Inspect and
        select Console to check for errors and warnings. Warnings can be
        ignored.

9.  Verify the Candid UI by opening the following URL in your web
    browser:

    ``` bash
    echo "$hello_world_backend_candid_url"
    ```

    a.  Verify UI loads, then test the greet function by entering text
        and clicking **Call** or clicking **Lucky**,

    b.  Verify there are no errors in the console by opening the
        Developer Tools.

        For example, in the browser, right-click, then click Inspect and
        select Console to check for errors and warnings. Warnings can be
        ignored.

10. Stop the replica by running the following command:

    ``` bash
    $dfx_rc stop
    ```

11. Delete the test project by running the following commands:

    ``` bash
    cd ..
    rm -rf hello_world
    ```

## Whitelist asset canister in Motoko Playground {#_whitelist_asset_canister_in_motoko_playground}

If the release includes a new version of the asset canister, then the
Motoko Playground needs to have the new asset canister WASM whitelisted.
Otherwise `dfx deploy --playground` will not work for asset canisters.

Find the new asset canister module hash. It will be listed in
`CHANGELOG.md` under `<n.n.n>` - `Dependencies` - `Frontend canister`.

Head over to the [Motoko Playground
repo](https://github.com/dfinity/motoko-playground) and create a PR that
adds the asset canister module hash to the whitelist in
`service/wasm-utils/lib.rs`. You can use [this
change](https://github.com/dfinity/motoko-playground/pull/175/files#diff-c8a035da9dcede5539deb0e81164ea50730e3177f56aef747d157406b1ba648dR15-R17)
as an example.

### Update the version {#_update_the_version}

1.  Set the new version in a temporary environment variable.

    For example, replace `<n.n.n>` with a specific version number:

    ``` nix-shell
    export NEW_DFX_VERSION=<n.n.n>
    ```

2.  If you're not already there, navigate back to the top-level of the
    `sdk` repo.

3.  Enter the sdk `nix` development environment by running the following
    command:

    ``` bash
    nix-shell --option extra-binary-caches https://cache.dfinity.systems
    ```

4.  Create a new branch for your changes by running the following
    command:

    ``` nix-shell
    git switch -c $USER/release-$NEW_DFX_VERSION
    ```

5.  Update the first `version` field in `src/dfx/Cargo.toml` to be equal
    to `$NEW_DFX_VERSION`

6.  Apply these changes to `Cargo.lock` by running the following
    command:

    ``` nix-shell
    cargo build
    ```

7.  Append the new version to `public/manifest.json` by appending it to
    the `versions` list.

    For example:

    ``` json
    {
        "tags": {
            "latest": "0.6.0"
        },
        "versions": [
            "0.5.15",
            "0.6.0",
            "n.n.n"
        ]
    }
    ```

    **Ensure** `tags.latest` remains the same.

8.  Exit `nix-shell` to continue.

### Create a pull request and tag {#_create_a_pull_request_and_tag}

1.  Create a pull request with the above changes by running the
    following commands:

    ``` bash
    git add --all
    git commit --signoff --message "chore: Release $NEW_DFX_VERSION"
    git push origin $USER/release-$NEW_DFX_VERSION
    ```

2.  After pushing, click the link in the console to go to your new
    branch in GitHub, then click **Create Pull Request**. Change the
    base branch to `release-<n.n.n>`.

3.  Have the validator review and approve the PR.

4.  Merge the PR manually (the automerge-squash label only works for PRs
    to the master branch).

    ::: note
    Depending on the number of jobs queued up, this step can take 45 to
    60 minutes to complete.
    :::

5.  Switch to the release branch by running the following command:

    ``` bash
    git switch release-$NEW_DFX_VERSION
    ```

6.  Set the upstream tracking information for the release branch:

    ``` bash
    git branch --set-upstream-to=origin/$NEW_DFX_VERSION $NEW_DFX_VERSION
    ```

7.  Update the release branch:

    ``` bash
    git pull
    ```

8.  Create a new tag by running the following command:

    ``` bash
    git tag --annotate $NEW_DFX_VERSION --message "Release: $NEW_DFX_VERSION"
    ```

9.  Verify the tag points to the correct version and includes annotation
    by running the following commands:

    ``` bash
    git log
    git describe --always
    ```

10. Push the tag by running the following command:

    ``` bash
    git push origin $NEW_DFX_VERSION
    ```

    The [publish.yml GitHub
    workflow](../../.github/workflows/publish.yml) will build the
    release and upload to GitHub releases after you push the tag.

### Add new frontend canister hash to list of WHITELISTED_WASMS in dfinity/motoko-playground repo {#_add_new_frontend_canister_hash_to_list_of_whitelisted_wasms_in_dfinitymotoko_playground_repo}

You can do it either by using GitHub UI
(<https://github.com/dfinity/sdk/actions/workflows/broadcast-frontend-hash.yml>)
or by running the following command:

``` bash
gh workflow run "broadcast-frontend-hash.yml" -f dfx_version=<n.n.n>
```

# Notification and post-build validation {#post-build}

1.  Wait for the publish workflow to complete.

2.  Install the build using the `DFX_VERSION=<version>` environment
    variable.

3.  Run through the [*Quick start - Local
    development*](https://sdk.dfinity.org/docs/quickstart/local-quickstart.html)
    steps.

4.  Run through [Check the connection to the
    network](https://sdk.dfinity.org/docs/quickstart/network-quickstart.html#ping-the-network)
    and [Register, build, and deploy the
    application](https://sdk.dfinity.org/docs/quickstart/network-quickstart.html#net-deploy)
    steps to deploy to the network.

5.  Notify [#eng-sdk](https://app.slack.com/client/T43F9UHS5/CGA566TPV)
    team members that the new build is ready for manual installation and
    testing.

    Remind the SDK and Apps teams to add information about *features and
    fixes* for release notes to their issues or PRs and to apply the
    changelog label to have the information included in the release
    notes.

6.  Notify the [Developer Forum](https://forum.dfinity.org) community if
    there are breaking changes.

    If a release is known to have changes that are not
    backward-compatible, create a forum post to describe the change and
    any instructions for migrating to the new release.

    Depending on the change, the notification might need to be posted in
    more than one topic channel. For example, changes to the external
    network for onboarded developers are currently posted in [Network
    status and
    updates](https://forum.dfinity.org/t/network-status-updates/928) on
    the [DFINITY Developer Forum](https://forum.dfinity.org).

# Promote a release candidate to production {#_promote_a_release_candidate_to_production}

1.  Verify that release notes and documentation are ready for public
    consumption.

2.  Open the `public/manifest.json` file in a text editor.

3.  Verify that `dfx deploy --playground` works with an asset canister
    by e.g. deploying the default project created by `dfx new`.

    a.  If it doesn't work, make sure the PR created on the Motoko
        Playground repo is merged and deployed.

4.  Under the `tags` key, change the version number associated with the
    `latest` key.

    For example:

    ``` json
    {
        "tags": {
            "latest": "n.n.n"
        }
    }
    ```

5.  Prepare a PR for the manifest by running the following commands:

    ``` bash
    git switch -c <YOUR_NAME>/update-n.n.n-latest
    git add --all
    git commit --message "chore: Update the manifest latest to n.n.n "
    git push origin <YOUR_NAME>/update-n.n.n-latest
    ```

6.  After pushing, click the link in the console to go to your new
    branch in GitHub, then click **Create Pull Request**.

7.  Have the validator review and approve the PR, then merge to
    `master`.

8.  Verify the Linux and Darwin (macOS) builds are available for
    download from
    https://github.com/dfinity/sdk/releases/download/n.n.n/dfx-n.n.n-architecture-os.tar.gz.

    Linux---Replace *n.n.n* with the new version number and
    *architecture-os* with `x86_64-linux`. For example, the following
    link would download version 0.6.1 for Linux:

    https://github.com/dfinity/sdk/releases/download/0.15.0/dfx-0.15.0-x86_64-linux.tar.gz\[\]

    Darwin (macOS)---Replace *n.n.n* with the new version number and
    *architecture-os* with `x86_64-darwin`. For example, the following
    link would download version 0.6.1 for macOS:

    https://github.com/dfinity/sdk/releases/download/0.15.0/dfx-0.15.0-x86_64-darwin.tar.gz\[\]

    Add a description and publish the tag for the latest release
    [https://github.com/dfinity-lab/sdk/releases/new?tag=\${NEW_DFX_VERSION}](https://github.com/dfinity-lab/sdk/releases/new?tag=${NEW_DFX_VERSION})

# Release documentation {#_release_documentation}

[Documentation repo](https://github.com/dfinity/docs)

1.  Tag the documentation using
    `git tag -a <version> -m <documentation-archive-message>`.

2.  Publish the tag on the remote server using
    `git push origin <tagname>`.

3.  Deploy updated documentation using Netlify.

# Requirements and properties {#_requirements_and_properties}

-   Semi-automation

-   Consistent delivery

-   Validation

-   Rollback

-   Guardrails

-   Flexibility

# Build mechanism {#_build_mechanism}

Github CI g == CI

CI release-related operation is split into two jobsets:

-   Generation and publishing of \'install.sh\' and \'manifest.json\'.

-   Tagging of a commit to release, building and publishing the
    necessary executables and files for supported platforms.

# Manifest {#_manifest}

We utilize a manifest to indicate to users (and in particular to our
installer and dfx executable) available and supported versions for
download. The manifest allows us to rollback a release or remove a
release from the list of supported releases. See [Version
Management](../specification/version_management.xml) for details on the
format of the manifest.

The manifest is generated when a patch is applied on master by the CI.

# Installer {#_installer}

The installer is generated when a patch is applied on the `master`
branch by the CI.

# Changelog {#_changelog}

A candidate changelog is generated automatically using the respective
tool (under scripts directory). Currently, the release notes are updated
manually in github.
