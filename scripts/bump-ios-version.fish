#!/usr/bin/env fish
# Sync iOS CFBundleShortVersionString to match Android appVersionName.
# Optionally bumps CFBundleVersion (build number).
#
# Usage:
#   scripts/bump-ios-version.fish                    # match Android, keep build number
#   scripts/bump-ios-version.fish --bump-build       # match Android, +1 build number
#   scripts/bump-ios-version.fish 0.20.0             # set explicit version

set -l repo_root (git rev-parse --show-toplevel)
set -l plist "$repo_root/mobile/iosApp/iosApp/Info.plist"
set -l gradle "$repo_root/mobile/androidApp/build.gradle.kts"

set -l target_version
set -l bump_build false

for arg in $argv
    switch $arg
        case --bump-build
            set bump_build true
        case '*'
            set target_version $arg
    end
end

if test -z "$target_version"
    set target_version (grep -E 'val appVersionName = "[^"]+"' $gradle | sed -E 's/.*"([^"]+)".*/\1/')
    if test -z "$target_version"
        echo "Could not extract appVersionName from $gradle" >&2
        exit 1
    end
end

# Use PlistBuddy on macOS, plistutil-free fallback (sed) on Linux.
if type -q /usr/libexec/PlistBuddy
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $target_version" $plist
    if test "$bump_build" = true
        set -l current_build (/usr/libexec/PlistBuddy -c "Print :CFBundleVersion" $plist)
        set -l next_build (math $current_build + 1)
        /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $next_build" $plist
        echo "iOS version → $target_version (build $next_build)"
    else
        echo "iOS version → $target_version"
    end
else
    # Linux: rewrite via sed. The Info.plist has predictable formatting from this repo.
    sed -i -E "/<key>CFBundleShortVersionString<\/key>/{n;s|<string>[^<]+</string>|<string>$target_version</string>|}" $plist
    if test "$bump_build" = true
        set -l current_build (grep -A1 '<key>CFBundleVersion</key>' $plist | tail -1 | sed -E 's|.*<string>([0-9]+)</string>.*|\1|')
        set -l next_build (math $current_build + 1)
        sed -i -E "/<key>CFBundleVersion<\/key>/{n;s|<string>[0-9]+</string>|<string>$next_build</string>|}" $plist
        echo "iOS version → $target_version (build $next_build)"
    else
        echo "iOS version → $target_version"
    end
end
