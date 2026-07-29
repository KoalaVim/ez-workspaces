## REMOVED Requirements

### Requirement: Reset to merge-base after checkout
**Reason**: The automatic reset made standard git operations (commit, push, rebase) confusing and forced users to manually undo the reset. Users expect the PR branch to be checked out normally with full commit history.
**Migration**: No action needed. Sessions created from PR URLs will now retain the branch as-is. Users who want dirty-file view can manually run `git reset --mixed $(git merge-base HEAD origin/<base>)`.

### Requirement: Merge-base resolution
**Reason**: Removed together with the reset-to-merge-base requirement since it only existed to support that feature.
**Migration**: None required.

### Requirement: Reset failure
**Reason**: Removed together with the reset-to-merge-base requirement since it only existed to handle reset errors.
**Migration**: None required.
