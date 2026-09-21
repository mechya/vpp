# Governance

VPP has one lead maintainer, **Gurung Bhupesh** ([@mechya](https://github.com/mechya)), the Licensor named in `LICENSE.md`, who has the final say. Area maintainers share the review work, but only the lead maintainer changes `main`.

## Roles

| Role | Can do | Chosen by |
|---|---|---|
| **Lead maintainer** | Merge pull requests. Make releases. Accept design documents. Decide format and API level changes, and anything about licensing or the CLA. | — |
| **Area maintainer** | Review and approve pull requests for the crates they own in `.github/CODEOWNERS`. They cannot merge. | The lead maintainer |
| **Triage helper** | Label, assign, and close issues. They cannot approve or merge. | The lead maintainer |
| **Contributor** | Fork the repository and open issues and pull requests, after signing the CLA (`CLA.md`). | Anyone |

A change flows: a contributor opens a pull request → an area maintainer approves it → the lead maintainer merges it.

## Becoming a maintainer

There is no fixed threshold. The lead maintainer invites people who have made several good contributions to one area and reviewed others' pull requests there helpfully. Area maintainers who are no longer active are moved back to contributor, with thanks, so `CODEOWNERS` stays accurate.

## Decisions

* **Small changes** are decided in the pull request.
* **Larger changes** need a design document first (`docs/design/`). Discussion happens on its issue, and the lead maintainer accepts or declines it with reasons.
* **Disagreements** that a pull request or issue cannot settle go to the lead maintainer, whose decision is final.

## How GitHub enforces this

* Area maintainers have Write access, because GitHub only counts approvals and code ownership from accounts with Write access.
* A ruleset on `main` turns on **Restrict updates** with only the lead maintainer on the bypass list, so nobody else can push or merge. It also requires a pull request, requires review from code owners, requires the CLA and CI checks, blocks force pushes, and restricts deletion.
* A tag ruleset lets only the lead maintainer create `v*` tags. Release signing keys stay with the lead maintainer.

## Changing this document

This makes the lead maintainer the bottleneck for merges. If that becomes a problem, area maintainers can be allowed to merge in their own crates, while `vpp-format`, `vpp-updater`, signing, and licensing stay with the lead maintainer. Changes to this document are made by the lead maintainer, in a pull request, so the history is public.
