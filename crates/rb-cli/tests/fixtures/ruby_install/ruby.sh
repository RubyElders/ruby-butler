#!/bin/sh
if [ "$1" = "--disable=gems" ]; then
    echo "bundled gems are unavailable" >&2
    exit 1
elif [ "$1" = "-rrbconfig" ]; then
    if [ -n "$RUBYGEMS_GEMDEPS$RUBYOPT$RUBYLIB" ]; then
        echo "external gem environment leaked into validation" >&2
        exit 1
    fi
    case "$GEM_HOME" in
        */lib/ruby/gems/3.4.0) ;;
        *) echo "validation did not select bundled gems" >&2; exit 1 ;;
    esac
    [ "$GEM_PATH" = "$GEM_HOME" ] || exit 1
    printf '%s' '3.4.10'
elif [ "$1" = "sentinel" ]; then
    printf '%s\n' 'Ruby command executed'
fi
