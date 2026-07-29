#!/bin/sh
# Remove what scripts/install.sh installed: the binary, the plugin host, the
# installed configuration, the translation catalogs, and — only when asked
# for — your own configuration file.
#
# Everything is decided first, shown as a plan, and confirmed once; only then
# is anything deleted. Ctrl-C during the questions leaves the machine exactly
# as it was.
#
# The rule is that this script removes what install.sh put there and nothing
# else. Catalogs are matched against the ones this tree ships, so a language
# file you wrote yourself is reported and left alone. Your own configuration
# is yours: it survives unless you pass --purge. Directories are removed only
# when they end up empty, and never the shared `bin` and `share` of a system
# prefix, which belong to the system rather than to dun.
#
# POSIX sh with no `local`, for the same reasons as install.sh: FreeBSD's
# base system has no bash and Solaris /bin/sh is ksh93.
set -eu

repo_root=$(cd "$(dirname "$0")/.." && pwd)

opt_prefix=""
opt_bin_dir=""
opt_config_dir=""
do_binary=yes
do_i18n=yes
do_config=yes
purge=no
force=no
dry_run=no
assume_yes=no

usage() {
    cat <<'USAGE'
Usage: scripts/uninstall.sh [options]

Removes what scripts/install.sh installed, from $HOME/.local by default:

  <prefix>/bin/dun, <prefix>/bin/dun-syntect-host
  <prefix>/share/dun/config  and the catalogs in <prefix>/share/dun/i18n
  ~/.config/dun/config       only with --purge — it is yours

Files you added yourself are reported, never deleted. The plan is shown and
confirmed before anything is removed.

Options:
  --prefix DIR      the prefix to remove from (default $HOME/.local)
  --bin-dir DIR     look for the binary here instead of <prefix>/bin
  --config-dir DIR  your configuration directory (default: dun's own
                    discovery — the directory of $DUN_CONFIG, else
                    $XDG_CONFIG_HOME/dun, else $HOME/.config/dun)
  --purge           also delete your own configuration file and its .bak
  --no-binary       leave the binary and the plugin in place
  --no-config       leave the installed configuration in place
  --no-i18n         leave the catalogs in place
  --force           remove the binary even when it does not identify itself
                    as dun (or cannot be run)
  --dry-run         print the plan and stop
  -y, --yes         do not ask anything
  -h, --help        show this text
USAGE
}

die() {
    printf 'uninstall.sh: %s\n' "$*" >&2
    exit 1
}

need_value() {
    [ "$2" -gt 1 ] || die "$1 needs a value (--help for usage)"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) need_value "$1" $#; opt_prefix=$2; shift ;;
        --prefix=*) opt_prefix=${1#--prefix=} ;;
        --bin-dir) need_value "$1" $#; opt_bin_dir=$2; shift ;;
        --bin-dir=*) opt_bin_dir=${1#--bin-dir=} ;;
        --config-dir) need_value "$1" $#; opt_config_dir=$2; shift ;;
        --config-dir=*) opt_config_dir=${1#--config-dir=} ;;
        --purge) purge=yes ;;
        --no-binary) do_binary=no ;;
        --no-config) do_config=no ;;
        --no-i18n) do_i18n=no ;;
        --force) force=yes ;;
        --dry-run) dry_run=yes ;;
        -y | --yes) assume_yes=yes ;;
        -h | --help) usage; exit 0 ;;
        *) die "unknown option $1 (--help for usage)" ;;
    esac
    shift
done

interactive=no
if [ "$assume_yes" = no ] && [ "$dry_run" = no ] && [ -t 0 ]; then
    interactive=yes
fi

# --- helpers ----------------------------------------------------------------

report() {
    printf '  %-9s %s\n' "$1" "$2"
}

note() {
    printf '  %-9s %s\n' "" "$1"
}

ask() {
    if [ "$interactive" = no ]; then
        [ "$2" = yes ]
        return $?
    fi
    if [ "$2" = yes ]; then
        printf '%s [Y/n] ' "$1"
    else
        printf '%s [y/N] ' "$1"
    fi
    ask_reply=""
    read -r ask_reply || ask_reply=""
    case "$ask_reply" in
        [Yy] | [Yy][Ee][Ss]) return 0 ;;
        [Nn] | [Nn][Oo]) return 1 ;;
        *) [ "$2" = yes ]; return $? ;;
    esac
}

remove_file() {
    rm -f "$1" || die "cannot remove $1 (a system prefix may need sudo)"
}

remove_dir() {
    rmdir "$1" || die "cannot remove $1"
}

# Entries in a directory, dotfiles included. The three globs are the portable
# idiom; an unmatched glob stays literal, so each candidate is tested.
count_entries() {
    ce_n=0
    for ce_entry in "$1"/* "$1"/.[!.]* "$1"/..?*; do
        if [ -e "$ce_entry" ] || [ -L "$ce_entry" ]; then
            ce_n=$((ce_n + 1))
        fi
    done
    printf '%s\n' "$ce_n"
}

# Does this file identify itself as dun? The binary is removed by path, and
# $HOME/.local/bin/dun could be somebody else's program with that name.
looks_like_dun() {
    lld_out=$("$1" --version 2>/dev/null) || return 1
    case "$lld_out" in
        'dun '*) return 0 ;;
        *) return 1 ;;
    esac
}

# --- where things are -------------------------------------------------------

if [ -z "$opt_prefix" ]; then
    if [ -n "$opt_bin_dir" ]; then
        opt_prefix=$(dirname "$opt_bin_dir")
    elif [ -n "${HOME:-}" ]; then
        opt_prefix=$HOME/.local
    else
        die "HOME is not set; pass --prefix DIR"
    fi
fi
[ -n "$opt_bin_dir" ] || opt_bin_dir=$opt_prefix/bin

bin_dir=$opt_bin_dir
share_dir=$opt_prefix/share/dun
installed_config=$share_dir/config
i18n_dir=$share_dir/i18n

if [ -n "$opt_config_dir" ]; then
    user_config=$opt_config_dir/config
elif [ -n "${DUN_CONFIG:-}" ]; then
    user_config=$DUN_CONFIG
elif [ -n "${XDG_CONFIG_HOME:-}" ]; then
    user_config=$XDG_CONFIG_HOME/dun/config
elif [ -n "${HOME:-}" ]; then
    user_config=$HOME/.config/dun/config
else
    user_config=""
fi

# The repository has i18n/ at the top; an unpacked --package tarball has the
# installed layout. Either is a valid list of "what we ship".
if [ -d "$repo_root/i18n" ]; then
    catalog_src=$repo_root/i18n
elif [ -d "$repo_root/share/dun/i18n" ]; then
    catalog_src=$repo_root/share/dun/i18n
else
    catalog_src=""
fi

# --- decide -----------------------------------------------------------------

plan_binary=absent
if [ "$do_binary" = yes ] && [ -e "$bin_dir/dun" ]; then
    if [ "$force" = no ] && ! looks_like_dun "$bin_dir/dun"; then
        plan_binary=foreign
    else
        plan_binary=remove
    fi
fi

plan_plugin=absent
if [ "$do_binary" = yes ] && [ -e "$bin_dir/dun-syntect-host" ]; then
    plan_plugin=remove
fi

plan_installed_config=absent
if [ "$do_config" = yes ] && [ -e "$installed_config" ]; then
    plan_installed_config=remove
fi

plan_i18n_gone=0
plan_i18n_left=0
if [ "$do_i18n" = yes ] && [ -d "$i18n_dir" ]; then
    [ -n "$catalog_src" ] || die "no catalog list in this tree to match against"
    plan_i18n_left=$(count_entries "$i18n_dir")
    for src in "$catalog_src"/*.conf; do
        [ -f "$src" ] || continue
        if [ -e "$i18n_dir/$(basename "$src")" ]; then
            plan_i18n_gone=$((plan_i18n_gone + 1))
        fi
    done
    plan_i18n_left=$((plan_i18n_left - plan_i18n_gone))
fi

plan_user_config=keep
if [ -n "$user_config" ] && [ -e "$user_config" ]; then
    if [ "$purge" = yes ]; then
        plan_user_config=remove
    fi
else
    plan_user_config=absent
fi

# Directories, decided from the counts rather than by trying and seeing: the
# plan has to be true before anything is deleted.
share_before=0
if [ -d "$share_dir" ]; then
    share_before=$(count_entries "$share_dir")
fi
share_gone=0
[ "$plan_installed_config" = remove ] && share_gone=$((share_gone + 1))
if [ -e "$installed_config.bak" ] && [ "$plan_installed_config" = remove ]; then
    share_gone=$((share_gone + 1))
fi
[ "$plan_i18n_left" -eq 0 ] && [ "$plan_i18n_gone" -gt 0 ] && share_gone=$((share_gone + 1))
plan_share_dir=keep
if [ "$share_before" -gt 0 ] && [ "$((share_before - share_gone))" -eq 0 ]; then
    plan_share_dir=remove
fi

user_dir=""
plan_user_dir=keep
if [ -n "$user_config" ]; then
    user_dir=$(dirname "$user_config")
    if [ -d "$user_dir" ]; then
        user_before=$(count_entries "$user_dir")
        user_gone=0
        if [ "$plan_user_config" = remove ]; then
            user_gone=$((user_gone + 1))
            [ -e "$user_config.bak" ] && user_gone=$((user_gone + 1))
        fi
        if [ "$((user_before - user_gone))" -eq 0 ] && [ "$user_gone" -gt 0 ]; then
            plan_user_dir=remove
        fi
    fi
fi

# A prefix whose last component is `dun` was made for dun alone (/opt/dun),
# so its emptied bin/ and share/ can go with it. A shared prefix like
# /usr/local is a different matter: an empty /usr/local/bin is still the
# system's directory and other tools expect it to exist.
plan_prefix=keep
case "$opt_prefix" in
    */dun) plan_prefix=maybe ;;
esac

# --- show the plan ----------------------------------------------------------

printf 'dun uninstall plan\n'
report prefix "$opt_prefix"

case "$plan_binary" in
    remove) report binary "$bin_dir/dun (remove)" ;;
    foreign) report binary "$bin_dir/dun (keep: does not identify as dun; --force removes it)" ;;
    absent) report binary "$bin_dir/dun (not there)" ;;
esac
if [ "$plan_plugin" = remove ]; then
    report plugin "$bin_dir/dun-syntect-host (remove)"
fi
case "$plan_installed_config" in
    remove) report config "$installed_config (remove)" ;;
    absent) report config "$installed_config (not there)" ;;
esac
if [ "$do_i18n" = yes ]; then
    if [ ! -d "$i18n_dir" ]; then
        report catalogs "$i18n_dir (not there)"
    elif [ "$plan_i18n_left" -gt 0 ]; then
        report catalogs "$i18n_dir (remove $plan_i18n_gone, keep $plan_i18n_left you added)"
    else
        report catalogs "$i18n_dir (remove $plan_i18n_gone, then the directory)"
    fi
fi
case "$plan_user_config" in
    remove) report personal "$user_config (remove — yours, --purge was given)" ;;
    keep) report personal "$user_config (keep; --purge deletes it)" ;;
    absent) ;;
esac
if [ "$plan_share_dir" = remove ]; then
    report directory "$share_dir (will be empty, remove)"
fi
if [ "$plan_user_dir" = remove ]; then
    report directory "$user_dir (will be empty, remove)"
fi
if [ "$plan_prefix" = maybe ]; then
    note "$opt_prefix is dun's own prefix: its bin/ and share/ go too if empty"
fi

printf '\n'

if [ "$dry_run" = yes ]; then
    printf 'Dry run: nothing was removed.\n'
    exit 0
fi

if [ "$interactive" = yes ]; then
    if ! ask "Proceed?" yes; then
        printf 'Nothing was removed.\n'
        exit 0
    fi
    printf '\n'
fi

# --- act --------------------------------------------------------------------

printf 'dun uninstall\n'
report prefix "$opt_prefix"

if [ "$plan_binary" = remove ]; then
    remove_file "$bin_dir/dun"
    report binary "$bin_dir/dun (removed)"
elif [ "$plan_binary" = foreign ]; then
    report binary "$bin_dir/dun (kept: does not identify as dun)"
fi

if [ "$plan_plugin" = remove ]; then
    remove_file "$bin_dir/dun-syntect-host"
    report plugin "$bin_dir/dun-syntect-host (removed)"
fi

if [ "$plan_installed_config" = remove ]; then
    remove_file "$installed_config"
    [ -e "$installed_config.bak" ] && remove_file "$installed_config.bak"
    report config "$installed_config (removed)"
fi

if [ "$do_i18n" = yes ] && [ ! -d "$i18n_dir" ]; then
    report catalogs "$i18n_dir (not there)"
elif [ "$do_i18n" = yes ]; then
    for src in "$catalog_src"/*.conf; do
        [ -f "$src" ] || continue
        dest=$i18n_dir/$(basename "$src")
        if [ -e "$dest" ]; then
            remove_file "$dest"
        fi
    done
    if [ "$plan_i18n_left" -gt 0 ]; then
        report catalogs "$i18n_dir (removed $plan_i18n_gone, kept $plan_i18n_left you added)"
    else
        remove_dir "$i18n_dir"
        report catalogs "$i18n_dir (removed $plan_i18n_gone and the directory)"
    fi
fi

if [ "$plan_user_config" = remove ]; then
    remove_file "$user_config"
    [ -e "$user_config.bak" ] && remove_file "$user_config.bak"
    report personal "$user_config (removed)"
elif [ "$plan_user_config" = keep ]; then
    report personal "$user_config (kept; it is yours)"
fi

if [ "$plan_share_dir" = remove ] && [ -d "$share_dir" ] &&
    [ "$(count_entries "$share_dir")" -eq 0 ]; then
    remove_dir "$share_dir"
    report directory "$share_dir (empty, removed)"
fi
if [ "$plan_user_dir" = remove ] && [ -d "$user_dir" ] &&
    [ "$(count_entries "$user_dir")" -eq 0 ]; then
    remove_dir "$user_dir"
    report directory "$user_dir (empty, removed)"
fi

if [ "$plan_prefix" = maybe ]; then
    prefix_emptied=no
    for prefix_dir in "$opt_prefix/bin" "$opt_prefix/share" "$opt_prefix"; do
        if [ -d "$prefix_dir" ] && [ "$(count_entries "$prefix_dir")" -eq 0 ]; then
            remove_dir "$prefix_dir"
            prefix_emptied=yes
        fi
    done
    if [ "$prefix_emptied" = yes ]; then
        report directory "$opt_prefix (dun's own prefix, emptied and removed)"
    fi
fi

# --- what is left behind ----------------------------------------------------

if [ -n "$user_config" ] && [ -e "$user_config" ] &&
    grep -q '^[ 	]*plugin\.' "$user_config" 2>/dev/null; then
    note "your configuration names plugin hosts; their files live wherever"
    note "you unpacked them, which is outside anything install.sh created"
fi
