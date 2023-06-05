This document describes the release process for `dfx`, with step-by-step
instructions, information about automation, and a checklist.

# Overview

Before starting the release process, the team should conduct a brief
Go/No-Go release review to evaluate the current state of fixes and
features ready to be included in a release candidate. After reviewing
the list of fixes and features, the team will decide whether to proceed
with staging a build.

For now, our release process is driven by the DFINITY foundation SDK
team. Future work needs to be done to separate the build and release
process to run fully on open systems.

If the new release is given the Go green light, two people who are
familiar with the process—a **driver** and a **validator**—use the steps
in this document to stage or promote a release candidate.

The **validator** should be the person most familiar with the process
and be able to assist with debugging or resolving issues if the
**driver** building the release runs into trouble.

A successful release is the result of coordination between automation,
manual steps performed by team members, and a validation process. Our
goal is to update this document with the latest information as we
iterate to improve the release process.

## Participants

-   Driver / Primary

-   Validator / Secondary

## Prerequisites

Before you begin, verify the following prerequisites for the release
process **driver**:

-   You must have a GitHub account and access to the `+dfinity+`
    organization and core repositories like the `sdk` repository

    For information about setting up a GitHub account, see [Signin up
    for a GitHub
    account](https://docs.github.com/en/github/getting-started-with-github/signing-up-for-a-new-github-account)
    To get permission for your account to access `+dfinity-lab+` and
    `+dfinity+` repositories, see
    link:https://www.notion.so/How-to-Get-Github-Access-68c9ad72b5974fa9bbec003592677d02\[How
    to get GitHub access). If you run into any issues accessing
    repositories, you can contact IT using the `#help-it` Slack channel
    and sending a message to `@firstresponder`.

-   You must have VPN access to the DFINITY VPN.

    If you don’t have a VPN connection set up for your computer, you’ll
    need to follow the instructions in [How to get VPN access to our
    data center
    services](https://www.notion.so/How-to-get-VPN-access-to-our-data-center-services-1c9b123152d740508eec25e7ac982259).

-   You must have [Nix installed and
    configured](https://github.com/dfinity-lab/dfinity/blob/master/CONTRIBUTING.adoc#install-and-configure-nix).

    If you are installing Nix for the first time, building the cache can
    take hours and might timeout depending on the reliability of your
    network connection.

-   You must have a registered NPM account and `npm` installed on your
    system.

    If you don’t have an account, you can go to the
    [NPMJS](https://www.npmjs.com/) website and click **Sign Up** to
    create one using your `firstname.lastname@dfinity.org` email
    address.

    You will need to verify your email address to complete the
    registration process. For your account to be added to the `dfinity`
    organization, contact IT

## Preliminary validation

Verify the general stability of the master branch before attempting to
create a release candidate.

1.  Use this [link](https://github.com/dfinity-lab/sdk/commits/master)
    to verify:

    1.  Is `master` green?

    2.  Was `master` red recently or flaky?

        ![](is-master-green.png)

## Preparation

1.  Connect to VPN.

2.  Open a terminal and `cd` into your local copy of the `sdk` repo.

## Creating a New Release Branch

1.  Check out the `master` branch and pull merged commits from the
    remote in your working directory by running the following command:

        git checkout master && git pull

2.  Create the release branch. Note that the branch name never includes
    alpha, beta, and so forth. All of these will be released on the same
    release branch.

        git switch -c release-<n.n.n> && git push

    For example, if creating the release branch for 0.7.0, you would run
    the following command:

        git switch -c release-0.7.0 && git push

3.  Edit CHANGELOG.md to remove the "UNRELEASED" note from the version
    to be released. Commit this to change to the release branch.

## Resuming on an Existing Release Branch

1.  Check out the `master` branch and pull merged commits from the
    remote in your working directory by running the following command:

        git checkout release-<n.n.n> && git pull

## Ready

At this point, you are ready to build a release candidate. There are two
ways you can build a release candidate:

-   Using the [SCRIPT-BASED release process](#script) to automate the
    build process and skip manual testing.

-   Using the [MANUAL STEPS release process](#manual) to build and test
    the release.

# SCRIPT-BASED release process

To use the release script to automate building the release candidate:

1.  Run the following command and substitute `<n.n.n>` with the version
    number for this release candidate:

        ./scripts/release.sh <n.n.n>

    For example, if releasing 0.7.0, you would run the following
    command:

        ./scripts/release.sh 0.7.0

2.  Follow the prompts displayed to complete the release process.

After running this command to build the release candidate, follow the
steps in [Notification and post-build validations](#post-build) to
complete the release process.

# MANUAL STEPS release process

The manual release process provides full instructions for building and
testing the release candidate binaries to ensure everything is working
before making a release available to internal or external developers.

## Build DFX

Verify that you can build DFX from the `+master+` branch without errors.

1.  Verify you’re connected to VPN, if necessary.

2.  Build the `dfx` binary by running the following command:

        cargo clean --release
        cargo build --release --locked
        export dfx_rc="$(pwd)/target/release/dfx"

    The `nix-build` command can take a while to complete. Wait for it to
    be done. These commands create the binary then stores the binary in
    a shell variable. . Make sure the `$dfx_rc` variable points to a
    real file by running the following command:

    \`\`\` test -x $dfx\_rc \\ && echo *Please proceed.* \\ || echo
    *Cant find executable $dfx\_rc*="$dfx\_rc" \`\`\`

    You should see *Please proceed* returned. . Delete the existing
    `dfx` cache to ensure you’re not using a stale binary.

        $dfx_rc cache delete

3.  Ensure `dfx` and `replica` are not running in the background by
    running the following command:

        ps -ef | grep -E 'replica|dfx' | grep -v grep

    If there are any `replica` or `dfx` processes running, use the
    `kill` command to terminate them.

## Validate the default project

Verify that you can build, deploy, and call the default `hello_world`
project without errors.

1.  Generate a default new project and change to the project directory
    by running the following commands:

        $dfx_rc new hello_world
        cd hello_world

2.  Start the local `replica` as a background process by running the
    following command:

        $dfx_rc start --clean --background

3.  Create, build, and install canisters by running the following
    command:

        $dfx_rc deploy

4.  Call the canister and verify the result by running the following
    command:

        $dfx_rc canister call hello_world greet everyone

5.  Save the canister URLs as shell variables, then print them by
    running the following commands:

        export hello_world_backend_candid_url="http://localhost:4943/candid?canisterId=$($dfx_rc canister id hello_world_backend)"
        export hello_world_frontend_url="http://localhost:4943/?canisterId=$($dfx_rc canister id hello_world_frontend)"

6.  Open a web browser and clear your cache or switch to Private
    Browsing/Incognito mode.

7.  Open the following URL in your web browser:

        echo "$hello_world_frontend_url"

8.  Verify that you are prompted to type a greeting in a prompt window.

    1.  Type a greeting, then click **OK** to return the greeting in an
        alert window.

    2.  Verify there are no errors in the console by opening the
        Developer Tools.

        For example, in the browser, right-click, then click Inspect and
        select Console to check for errors and warnings. Warnings can be
        ignored.

9.  Verify the Candid UI by opening the following URL in your web
    browser:

        echo "$hello_world_backend_candid_url"

    1.  Verify UI loads, then test the greet function by entering text
        and clicking **Call** or clicking **Lucky**,

    2.  Verify there are no errors in the console by opening the
        Developer Tools.

        For example, in the browser, right-click, then click Inspect and
        select Console to check for errors and warnings. Warnings can be
        ignored. . Stop the replica by running the following command:

            $dfx_rc stop

10. Delete the test project by running the following commands:

        cd ..
        rm -rf hello_world

### Update the version

1.  Set the new version in a temporary environment variable.

    For example, replace `<n.n.n>` with a specific version number:

        export NEW_DFX_VERSION=<n.n.n>

2.  If you’re not already there, navigate back to the top-level of the
    `sdk` repo.

3.  Enter the sdk `nix` development environment by running the following
    command:

        nix-shell --option extra-binary-caches https://cache.dfinity.systems

4.  Create a new branch for your changes by running the following
    command:

        git switch -c $USER/release-$NEW_DFX_VERSION

5.  Update the first `version` field in `src/dfx/Cargo.toml` to be equal
    to `$NEW_DFX_VERSION`

6.  Apply these changes to `Cargo.lock` by running the following
    command:

        cargo build

7.  Append the new version to `public/manifest.json` by appending it to
    the `versions` list.

    For example:

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

    **Ensure** `tags.latest` remains the same. . Exit `nix-shell` to
    continue.

### Create a pull request and tag

1.  Create a pull request with the above changes by running the
    following commands:

        git add --all
        git commit --signoff --message "chore: Release $NEW_DFX_VERSION"
        git push origin $USER/release-$NEW_DFX_VERSION

2.  After pushing, click the link in the console to go to your new
    branch in GitHub, then click **Create Pull Request**. Change the
    base branch to `release-<n.n.n>`.

3.  Have the validator review and approve the PR.

4.  Merge the PR manually (the automerge-squash label only works for PRs
    to the master branch).

    Depending on the number of jobs queued up, this step can take 45 to
    60 minutes to complete.

5.  Switch to the release branch by running the following command:

        git switch release-$NEW_DFX_VERSION

6.  Set the upstream tracking information for the release branch:

        git branch --set-upstream-to=origin/$NEW_DFX_VERSION $NEW_DFX_VERSION

7.  Update the release branch:

        git pull

8.  Create a new tag by running the following command:

        git tag --annotate $NEW_DFX_VERSION --message "Release: $NEW_DFX_VERSION"

9.  Verify the tag points to the correct version and includes annotation
    by running the following commands:

        git log
        git describe --always

10. Push the tag by running the following command:

        git push origin $NEW_DFX_VERSION

    The [publish.yml GitHub
    workflow](../../.github/workflows/publish.yml) will build the
    release and upload to GitHub releases after you push the tag.

# Notification and post-build validation

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

5.  Notify [\#eng-sdk](https://app.slack.com/client/T43F9UHS5/CGA566TPV)
    team members that the new build is ready for manual installation and
    testing.

    Remind the SDK and Apps teams to add information about *features and
    fixes* for release notes to their issues or PRs and to apply the
    changelog label to have the information included in the release
    notes. . Notify the [Developer Forum](https://forum.dfinity.org)
    community if there are breaking changes.

    If a release is known to have changes that are not
    backward-compatible, create a forum post to describe the change and
    any instructions for migrating to the new release.

    Depending on the change, the notification might need to be posted in
    more than one topic channel. For example, changes to the external
    network for onboarded developers are currently posted in [Network
    status and
    updates](https://forum.dfinity.org/t/network-status-updates/928) on
    the [DFINITY Developer Forum](https://forum.dfinity.org).

# Promote a release candidate to production

1.  Verify that release notes and documentation are ready for public
    consumption.

2.  Open the `public/manifest.json` file in a text editor.

3.  Under the `tags` key, change the version number associated with the
    `latest` key.

    For example:

        {
            "tags": {
                "latest": "n.n.n"
            }
        }

4.  Prepare a PR for the manifest by running the following commands:

        git switch -c <YOUR_NAME>/update-n.n.n-latest
        git add --all
        git commit --message "chore: Update the manifest latest to n.n.n "
        git push origin <YOUR_NAME>/update-n.n.n-latest

5.  After pushing, click the link in the console to go to your new
    branch in GitHub, then click **Create Pull Request**.

6.  Have the validator review and approve the PR, then merge to
    `master`.

7.  Verify the Linux and Darwin (macOS) builds are available for
    download from
    https://download.dfinity.systems/sdk/dfx/n.n.n/architecture/dfx-n.n.n.tar.gz.

    Linux—Replace *n.n.n* with the new version number and *architecture*
    with `x86_64-linux`. For example, the following link would download
    version 0.6.1 for Linux:

    <https://download.dfinity.systems/sdk/dfx/0.6.1/x86_64-linux/dfx-0.6.1.tar.gz>\[\]

    Darwin (macOS)—Replace *n.n.n* with the new version number and
    *architecture* with `x86_64-darwin`. For example, the following link
    would download version 0.6.1 for macOS:

    <https://download.dfinity.systems/sdk/dfx/0.6.1/x86_64-darwin/dfx-0.6.1.tar.gz>\[\]

    CI Hydra:

    <https://hydra.dfinity.systems/jobset/dfinity-ci-build/sdk-release>

Add a description and publish the tag for the latest release

# Release documentation

[Documentation repo](https://github.com/dfinity/docs)

1.  Tag the documentation using
    `git tag -a <version> -m <documentation-archive-message>`.

2.  Publish the tag on the remote server using
    `git push origin <tagname>`.

3.  Deploy updated documentation using Netlify.

# Requirements and properties

-   Semi-automation

-   Consistent delivery

-   Validation

-   Rollback

-   Guardrails

-   Flexibility

# Build mechanism

Our build process is described in the `release.nix` derivation. The
`release.nix` derivation mainly invokes the `dfx-release` derivation
passing the annotated tag on HEAD (which happens right now to be the
stable branch). The `dfx-release` derivation builds the release binaries
and files for each platform and generates a manifest for S3 that
includes the tag name. The release tag allows us to keep a directory
structure with all past and upcoming releases in S3.

# CI

CI release-related operation is split into two jobsets:

-   Generation and publishing of *install.sh* and *manifest.json*.

-   Tagging of a commit to release, building and publishing the
    necessary executables and files for supported platforms.

# Manifest

We utilize a manifest to indicate to users (and in particular to our
installer and dfx executable) available and supported versions for
download. The manifest allows us to rollback a release or remove a
release from the list of supported releases. See [Version
Management](../specification/version_management.xml) for details on the
format of the manifest.

The manifest is generated when a patch is applied on master by the CI.

# Installer

The installer is generated when a patch is applied on the `master`
branch by the CI.

# Changelog

A candidate changelog is generated automatically using the respective
tool (under scripts directory). Currently, the release notes are updated
manually in github.

# Publishing of artifacts

We now summarize the release process. Our first step is to ensure the
proper and valid state of the `master` branch. Next, we update `cargo`
and the manifest accordingly. We then create and push an annotated tag
on the `stable` branch, generate the changelog. The product and SDK team
members can then inspect, clarify, and develop the changelog to ensure
it is appropriate for public consumption. After ensuring the proper
artifacts are available in S3, we can now publish them by updating the
manifest.

# TODOs and improvements

1.  version from the tag

2.  release stress tests

3.  valid json test for the manifest