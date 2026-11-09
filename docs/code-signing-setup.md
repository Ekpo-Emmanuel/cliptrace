# Code signing with SignPath Foundation

Cliptrace is not signed yet, so Windows SmartScreen warns about an unknown publisher. This is the plan for fixing that with the free [SignPath Foundation](https://signpath.org) program for open source projects. Everything here is based on SignPath's own documents as read on 2026-11-08. They can change, so check [signpath.org/terms](https://signpath.org/terms) and [docs.signpath.io](https://docs.signpath.io) when you act on it.

## What signing will and will not do

- The signed installer shows "SignPath Foundation" as the publisher, not a personal name. The Foundation owns the certificate and Cliptrace borrows it.
- SmartScreen builds trust from downloads over time, so a newly signed file can still warn for a while.
- Only the installer and the app exe are signed. SignPath's documents show signing the files inside a container only for MSI, not NSIS, so the exe inside the installer is not signed separately.

## What the program asks of the project

- An OSI-approved license without commercial dual licensing (MIT: met). No proprietary components. ONNX Runtime is MIT and ships unsigned-by-us inside the package, which the terms allow.
- Already released in the form that should be signed (met), with a download page that describes what the app does (met: the landing page).
- Builds run on GitHub-hosted runners, verifiably from the source in this repository. That is what `.github/workflows/release.yml` is for.
- Two-factor authentication on SignPath and on the GitHub account.
- A team with roles: Authors, Reviewers, and Approvers. Every release is approved by hand in SignPath before it is signed.
- An uninstaller (met: the NSIS installer has one).
- A **code signing policy** published on the project home page and on the download page, and a privacy statement. See the text below.

## Steps

1. **You:** turn on two-factor authentication for the GitHub account `Ekpo-Emmanuel`, if it is not on.
2. **You:** apply at <https://signpath.org/apply>. Acceptance is at the Foundation's discretion and no approval time is published. If the form asks for details that are not in this document, put them here so the next step can use them.
3. **You, after approval:** in SignPath, create a project and note its slug. Set the repository URL to `https://github.com/Ekpo-Emmanuel/cliptrace`. Link the GitHub.com trusted build system to the project.
4. **You:** add the artifact configuration from `.signpath/artifact-configuration.xml`, and add the product name and version restrictions its comment mentions.
5. **You:** create a signing policy for releases. Turn on the approval process, turn on trusted build system verification and origin verification, and allow the branch `main`.
6. **You:** in the GitHub repository settings, add these repository **variables**: `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_SIGNING_POLICY_SLUG`. Add this **secret**: `SIGNPATH_API_TOKEN`, from a SignPath user who may submit signing requests for that policy.
7. **Together:** publish the code signing policy (below) in the README, on the landing page, and in the release notes.
8. To release: bump the version, merge to `main`, then run the **Release** workflow from `main` in the Actions tab. When SignPath asks, approve the signing request. The workflow leaves a draft release. Write the notes and publish it.

Until step 6 is done the workflow still runs, but it skips signing and its draft release says the build is not signed. That lets the build be tested now.

## Text to publish after approval

Do not publish this before the Foundation accepts the project, because it would not be true yet.

> ### Code signing policy
>
> Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).
>
> Cliptrace is built from the source in this repository on GitHub-hosted runners and signed by the SignPath Foundation certificate. Every release is approved by hand before it is signed.
>
> - Authors, reviewers and approvers: Emmanuel Ekpo ([@Ekpo-Emmanuel](https://github.com/Ekpo-Emmanuel)). Other contributors are added here and in SignPath when they join.
>
> ### Privacy
>
> Cliptrace keeps your clipboard history on your PC and never sends it anywhere. The only network request it makes is a one-time download of the search model on first run, which sends none of your data. Everything else works offline.

The Foundation's terms offer a stock sentence for the privacy statement ("will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it"). It is not used above because the model download happens without the user asking for it, so the stock sentence would not be exactly true.

## Open questions to settle with SignPath

- Whether a build started by `workflow_dispatch` on `main` satisfies origin verification as written here. It should, since the branch is `main`, but I could not read the part of their docs about tags and manual runs.
- Whether the free program accepts an NSIS installer as the main signed file. Their examples use MSI. If it works as separate files, as assumed here, nothing changes.
