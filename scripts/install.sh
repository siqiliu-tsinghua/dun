#!/bin/sh
# Install dun: the binary, the syntax-highlighting plugin, the configuration
# it runs on, and the translation catalogs.
#
# Building dun produces one executable and nothing else. That leaves a first
# run with no configuration file and — the part nobody guesses — no catalogs,
# so the interface stays English however `LANG` is set. This script does the
# copies the user guide would otherwise ask you to make by hand, and then
# reports which catalog your locale actually selects, which is the one
# question a manual copy leaves open.
#
# Everything is decided first, shown as a plan, and confirmed once; only then
# does anything get written. Ctrl-C during the questions leaves the machine
# exactly as it was.
#
# The layout, per-user by default and system-wide with --prefix:
#
#   <prefix>/bin/dun                  the editor
#   <prefix>/bin/dun-syntect-host     the highlighting plugin
#   <prefix>/share/dun/config         the installed configuration
#   <prefix>/share/dun/i18n/*.conf    the translation catalogs
#   ~/.config/dun/config              yours, applied on top of the above
#
# dun reads the installed configuration first and your own file over it, key
# by key, and looks for catalogs beside your config file before falling back
# to <bin>/../share/dun/i18n. That is what makes one installation serve every
# user on a machine while each of them keeps their own settings.
#
# POSIX sh with no `local`, for the same reason as release-build.sh: FreeBSD's
# base system has no bash and Solaris /bin/sh is ksh93, which has no `local`.
set -eu

repo_root=$(cd "$(dirname "$0")/.." && pwd)

opt_prefix=""
opt_bin_dir=""
opt_config_dir=""
opt_dun=""
opt_langs=""
opt_syntect=""
opt_path_setup=""
opt_package=""
do_binary=yes
do_config=yes
do_i18n=yes
force=no
dry_run=no
assume_yes=no

usage() {
    cat <<'USAGE'
Usage: scripts/install.sh [options]

Installs, into $HOME/.local by default:

  <prefix>/bin/dun                  the editor
  <prefix>/bin/dun-syntect-host     the syntax-highlighting plugin
  <prefix>/share/dun/config         the installed configuration, with the
                                    plugin enabled in it
  <prefix>/share/dun/i18n/*.conf    the translation catalogs
  ~/.config/dun/config              yours, applied on top of the installed
                                    one key by key

Asks where to install, whether to enable the plugin and whether to fix your
PATH when stdin is a terminal, then shows the whole plan and asks once more
before writing anything.

Options:
  --prefix DIR      install here (default $HOME/.local; try /usr/local or
                    /opt/dun for a machine-wide install, which needs root)
  --bin-dir DIR     put the binary here instead of <prefix>/bin
  --config-dir DIR  your own configuration directory (default: dun's own
                    discovery — the directory of $DUN_CONFIG, else
                    $XDG_CONFIG_HOME/dun, else $HOME/.config/dun)
  --dun PATH        the binary to install and to dump the configuration from
  --lang LIST       comma-separated catalogs instead of all of them,
                    e.g. --lang zh-Hans,ja
  --syntect         install the syntect plugin host and enable it
  --no-syntect      leave the plugin out
  --path-setup      add the bin directory to PATH in your shell's rc file
  --no-path-setup   only print the line to add
  --package FILE    do not install: write a tarball of the binary, the
                    plugin, the catalogs and these scripts, for unpacking
                    and installing on another machine of the same platform
  --no-binary       do not install the binary or the plugin
  --no-config       do not write either configuration file
  --no-i18n         do not copy any catalog
  --force           overwrite existing files; an existing configuration file
                    is copied to config.bak first
  --dry-run         print the plan and stop
  -y, --yes         do not ask anything; take the defaults
  -h, --help        show this text
USAGE
}

die() {
    printf 'install.sh: %s\n' "$*" >&2
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
        --dun) need_value "$1" $#; opt_dun=$2; shift ;;
        --dun=*) opt_dun=${1#--dun=} ;;
        --lang) need_value "$1" $#; opt_langs=$2; shift ;;
        --lang=*) opt_langs=${1#--lang=} ;;
        --package) need_value "$1" $#; opt_package=$2; shift ;;
        --package=*) opt_package=${1#--package=} ;;
        --syntect) opt_syntect=yes ;;
        --no-syntect) opt_syntect=no ;;
        --path-setup) opt_path_setup=yes ;;
        --no-path-setup) opt_path_setup=no ;;
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

if [ -n "$opt_langs" ]; then
    do_i18n=yes
fi

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

# ask "question" default(yes|no) -> 0 for yes
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

# Absolute path, resolved through the parent directory when that exists, so
# two spellings of the same destination compare equal.
abs_path() {
    ap_dir=$(dirname "$1")
    if [ -d "$ap_dir" ]; then
        printf '%s/%s\n' "$(cd "$ap_dir" && pwd)" "$(basename "$1")"
    else
        printf '%s\n' "$1"
    fi
}

ensure_dir() {
    if [ -d "$1" ]; then
        return 0
    fi
    mkdir -p "$1" || die "cannot create $1 (a system prefix may need sudo)"
}

# Copy through a temporary name in the destination directory, so an
# interrupted run cannot leave a half-written file where dun will read one,
# and so replacing a running binary does not fail with ETXTBSY.
copy_file() {
    cp "$1" "$2.tmp.$$" || die "cannot write $2 (a system prefix may need sudo)"
    chmod "$3" "$2.tmp.$$"
    mv "$2.tmp.$$" "$2" || die "cannot write $2"
}

# --- what this tree has to offer --------------------------------------------

# The repository has i18n/ at the top; a --package tarball has the installed
# layout instead. Both are handled so the same script runs on both sides of
# an scp.
if [ -d "$repo_root/i18n" ]; then
    catalog_src=$repo_root/i18n
elif [ -d "$repo_root/share/dun/i18n" ]; then
    catalog_src=$repo_root/share/dun/i18n
else
    catalog_src=""
fi

syntect_src=""
if [ -x "$repo_root/hosts/rust-syntect/target/release/dun-syntect-host" ]; then
    syntect_src=$repo_root/hosts/rust-syntect/target/release/dun-syntect-host
elif [ -x "$repo_root/bin/dun-syntect-host" ]; then
    syntect_src=$repo_root/bin/dun-syntect-host
fi

dun_bin=""
find_dun() {
    if [ -n "$opt_dun" ]; then
        [ -x "$opt_dun" ] || die "$opt_dun is not an executable file"
        dun_bin=$opt_dun
        return 0
    fi
    for fd_candidate in "$repo_root/bin/dun" "$repo_root/target/release/dun"; do
        if [ -x "$fd_candidate" ]; then
            dun_bin=$fd_candidate
            return 0
        fi
    done
    fd_triple=""
    if command -v rustc >/dev/null 2>&1; then
        fd_triple=$(rustc -vV | sed -n 's/^host: //p')
    fi
    if [ -n "$fd_triple" ] && [ -x "$repo_root/target/$fd_triple/release/dun" ]; then
        dun_bin=$repo_root/target/$fd_triple/release/dun
        return 0
    fi
    dun_bin=$(command -v dun 2>/dev/null || true)
    if [ -n "$dun_bin" ]; then
        return 0
    fi
    die "no dun binary found. Build one first:
    scripts/build.sh
then run this again, or point it at a binary with --dun PATH."
}

lang_selected() {
    if [ -z "$opt_langs" ]; then
        return 0
    fi
    ls_rest=$opt_langs
    while [ -n "$ls_rest" ]; do
        ls_head=${ls_rest%%,*}
        case "$ls_rest" in
            *,*) ls_rest=${ls_rest#*,} ;;
            *) ls_rest="" ;;
        esac
        if [ "$ls_head" = "$1" ]; then
            return 0
        fi
    done
    return 1
}

available_tags() {
    for at_file in "$catalog_src"/*.conf; do
        [ -f "$at_file" ] || continue
        at_tag=$(basename "$at_file")
        printf '%s ' "${at_tag%.conf}"
    done
    printf '\n'
}

if [ -n "$opt_langs" ]; then
    [ -n "$catalog_src" ] || die "no catalogs in this tree to choose from"
    check_rest=$opt_langs
    while [ -n "$check_rest" ]; do
        check_tag=${check_rest%%,*}
        case "$check_rest" in
            *,*) check_rest=${check_rest#*,} ;;
            *) check_rest="" ;;
        esac
        [ -n "$check_tag" ] || continue
        [ -f "$catalog_src/$check_tag.conf" ] ||
            die "no catalog named $check_tag. Available: $(available_tags)"
    done
fi

# --- package mode: build a tarball and stop ---------------------------------

if [ -n "$opt_package" ]; then
    find_dun
    [ -n "$catalog_src" ] || die "no catalogs found to package"
    version=$("$dun_bin" --version | cut -d' ' -f2)
    # `uname -m` answers `i86pc` on Solaris x86 — the platform name, not the
    # instruction set, which would make a 64-bit tarball look 32-bit. The
    # kernel ISA is the honest answer there.
    arch=$(uname -m)
    if [ "$(uname -s)" = SunOS ] && command -v isainfo >/dev/null 2>&1; then
        arch=$(isainfo -k)
    fi
    platform=$(uname -s | tr '[:upper:]' '[:lower:]')-$arch
    name=dun-$version-$platform
    stage=${TMPDIR:-/tmp}/dun-package-$$
    trap 'rm -rf "$stage"' EXIT INT TERM

    printf 'dun package plan\n'
    report package "$opt_package"
    report contents "$name/bin/dun"
    if [ -n "$syntect_src" ]; then
        note "$name/bin/dun-syntect-host"
    fi
    note "$name/share/dun/i18n/*.conf"
    note "$name/scripts/{install,uninstall}.sh"
    printf '\n'
    if [ "$dry_run" = yes ]; then
        exit 0
    fi
    if [ "$interactive" = yes ] && ! ask "Write it?" yes; then
        printf 'Nothing was written.\n'
        exit 0
    fi

    mkdir -p "$stage/$name/bin" "$stage/$name/share/dun/i18n" "$stage/$name/scripts"
    cp "$dun_bin" "$stage/$name/bin/dun"
    chmod 755 "$stage/$name/bin/dun"
    if [ -n "$syntect_src" ]; then
        cp "$syntect_src" "$stage/$name/bin/dun-syntect-host"
        chmod 755 "$stage/$name/bin/dun-syntect-host"
    fi
    for src in "$catalog_src"/*.conf; do
        [ -f "$src" ] || continue
        tag=$(basename "$src")
        tag=${tag%.conf}
        lang_selected "$tag" || continue
        cp "$src" "$stage/$name/share/dun/i18n/"
    done
    for script in install.sh uninstall.sh; do
        cp "$repo_root/scripts/$script" "$stage/$name/scripts/$script"
        chmod 755 "$stage/$name/scripts/$script"
    done
    cat > "$stage/$name/INSTALL.txt" <<EOF
dun $version for $platform

  scripts/install.sh                      install into \$HOME/.local
  scripts/install.sh --prefix /opt/dun    install system-wide (needs root)
  scripts/install.sh --help               everything else

The binary runs only on $platform. The installed configuration and the
catalogs are found through the binary's own location (<bin>/../share/dun),
so the prefix can be anywhere; your personal settings go in
~/.config/dun/config and win over the installed ones.
EOF
    # `tar cf - | gzip` rather than `tar czf`: Solaris tar has no -z.
    (cd "$stage" && tar cf - "$name") | gzip -9 > "$opt_package" ||
        die "cannot write $opt_package"

    printf 'dun package\n'
    report package "$opt_package"
    note "on the target host: gzip -dc $(basename "$opt_package") | tar xf -"
    note "then: $name/scripts/install.sh"
    exit 0
fi

# --- decide: where ----------------------------------------------------------

if [ "$interactive" = yes ] && [ -z "$opt_prefix" ] && [ -z "$opt_bin_dir" ]; then
    printf 'Where should dun be installed?\n'
    printf '  1) %s/.local — just for you, no root needed (default)\n' "${HOME:-~}"
    printf '  2) /usr/local — everyone on this machine, needs root\n'
    printf '  3) /opt/dun   — everyone on this machine, self-contained, needs root\n'
    printf '  4) somewhere else\n'
    printf 'Choice [1] '
    layout_reply=""
    read -r layout_reply || layout_reply=""
    case "$layout_reply" in
        2) opt_prefix=/usr/local ;;
        3) opt_prefix=/opt/dun ;;
        4)
            printf 'Prefix (its bin/ and share/ are used): '
            read -r layout_reply || layout_reply=""
            [ -n "$layout_reply" ] || die "no prefix given"
            opt_prefix=$layout_reply
            ;;
        *) ;;
    esac
    printf '\n'
fi

if [ -z "$opt_prefix" ]; then
    if [ -n "$opt_bin_dir" ]; then
        # A bin directory without a prefix still implies one: dun finds its
        # own share/dun through <bin>/.., so they cannot be chosen apart.
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
syntect_dest=$bin_dir/dun-syntect-host

# Mirrors dun's own discovery order (crates/dun-cli/src/config_loading.rs):
# DUN_CONFIG names the config *file*, so its directory is the config
# directory.
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

# Under sudo the personal file would land in root's home, which is nobody's
# configuration. Install the machine-wide parts and say what is left.
sudo_note=no
if [ "$(id -u)" = 0 ] && [ -n "${SUDO_USER:-}" ] && [ -z "$opt_config_dir" ]; then
    user_config=""
    sudo_note=yes
fi

if [ "$do_binary" = yes ] || [ "$do_config" = yes ]; then
    find_dun
fi

# --- decide: the plugin -----------------------------------------------------

if [ -z "$opt_syntect" ]; then
    if [ -z "$syntect_src" ] || [ "$do_binary" = no ]; then
        opt_syntect=no
    elif [ "$interactive" = yes ]; then
        if ask "Install the syntect highlighting plugin and enable it?" yes; then
            opt_syntect=yes
        else
            opt_syntect=no
        fi
    else
        opt_syntect=yes
    fi
fi
if [ "$opt_syntect" = yes ] && [ -z "$syntect_src" ]; then
    die "no syntect host built. Build one with:
    scripts/build.sh --syntect
or pass --no-syntect."
fi

# --- decide: PATH -----------------------------------------------------------

# The rc file this user's shell actually reads, best effort. Reported either
# way, so a wrong guess is visible rather than silent.
rc_file_for_shell() {
    case "${SHELL:-}" in
        */zsh) printf '%s\n' "${ZDOTDIR:-$HOME}/.zshrc" ;;
        */bash)
            if [ -f "$HOME/.bashrc" ]; then
                printf '%s\n' "$HOME/.bashrc"
            else
                printf '%s\n' "$HOME/.bash_profile"
            fi
            ;;
        */ksh) printf '%s\n' "$HOME/.kshrc" ;;
        *) printf '%s\n' "$HOME/.profile" ;;
    esac
}

path_line=""
path_line_dir=""
rc_file=""
path_action=none          # none | append | print | present
if [ "$do_binary" = yes ]; then
    case ":${PATH:-}:" in
        *":$bin_dir:"*) path_action=present ;;
        *)
            # Keep $HOME symbolic in the rc file: dotfiles travel.
            path_line_dir=$bin_dir
            case "$bin_dir" in
                "${HOME:-/dev/null}"/*) path_line_dir="\$HOME${bin_dir#${HOME:-}}" ;;
            esac
            path_line="export PATH=\"$path_line_dir:\$PATH\""
            if [ -n "${HOME:-}" ]; then
                rc_file=$(rc_file_for_shell)
            fi
            path_action=print
            if [ -n "$rc_file" ]; then
                if [ -z "$opt_path_setup" ]; then
                    if [ "$interactive" = yes ] &&
                        ask "$bin_dir is not on your PATH. Add it to $rc_file?" yes; then
                        path_action=append
                    fi
                elif [ "$opt_path_setup" = yes ]; then
                    path_action=append
                fi
                if [ "$path_action" = append ] &&
                    grep -q "$path_line_dir" "$rc_file" 2>/dev/null; then
                    path_action=present
                fi
            fi
            ;;
    esac
fi

# --- decide: every file, before writing any of them -------------------------

plan_binary=skip
if [ "$do_binary" = yes ]; then
    if [ "$(abs_path "$dun_bin")" = "$(abs_path "$bin_dir/dun")" ]; then
        plan_binary=inplace
    else
        plan_binary=install
    fi
fi

plan_plugin=skip
if [ "$opt_syntect" = yes ]; then
    plan_plugin=install
fi

# The installed configuration is the full template; the personal one is a
# commented stub, because a copy of every default in a file meant for
# overrides would bury the two lines the user actually changed.
plan_installed_config=skip
plan_stanza=no
if [ "$do_config" = yes ]; then
    if [ ! -e "$installed_config" ] || [ "$force" = yes ]; then
        plan_installed_config=write
        if [ "$opt_syntect" = yes ]; then
            plan_stanza=inline
        fi
    else
        plan_installed_config=keep
        if [ "$opt_syntect" = yes ] &&
            ! grep -q '^[ 	]*plugin\.syntect\.' "$installed_config" 2>/dev/null; then
            if [ "$interactive" = yes ] &&
                ask "Enable the plugin in $installed_config (3 lines appended)?" yes; then
                plan_stanza=append
            else
                plan_stanza=print
            fi
        fi
    fi
fi

plan_user_config=skip
if [ "$do_config" = yes ] && [ -n "$user_config" ]; then
    if [ ! -e "$user_config" ]; then
        plan_user_config=write
    else
        plan_user_config=keep
    fi
fi

plan_i18n_new=0
plan_i18n_kept=0
if [ "$do_i18n" = yes ]; then
    [ -n "$catalog_src" ] || die "no catalogs in this tree to install"
    for src in "$catalog_src"/*.conf; do
        [ -f "$src" ] || continue
        tag=$(basename "$src")
        tag=${tag%.conf}
        lang_selected "$tag" || continue
        if [ -e "$i18n_dir/$tag.conf" ] && [ "$force" = no ]; then
            plan_i18n_kept=$((plan_i18n_kept + 1))
        else
            plan_i18n_new=$((plan_i18n_new + 1))
        fi
    done
fi

# --- decide: which catalog the locale will select ---------------------------

# The candidate chain of docs/i18n.md, mirrored: region, then script, then
# bare language. This is a report, not the decision — dun makes that itself
# in dun-config's locale_candidates.
locale_raw=""
locale_var=""
for var in LC_ALL LC_MESSAGES LANG; do
    eval "value=\${$var:-}"
    if [ -n "$value" ]; then
        locale_raw=$value
        locale_var=$var
        break
    fi
done

print_candidates() {
    pc_base=${1%%.*}
    pc_base=${pc_base%%@*}
    case "$pc_base" in
        "" | [Cc] | [Pp][Oo][Ss][Ii][Xx]) return 0 ;;
    esac
    pc_primary=$(printf '%s' "${pc_base%%[_-]*}" | tr '[:upper:]' '[:lower:]')
    case "$pc_primary" in
        "" | *[!a-z]*) return 0 ;;
    esac
    case "$pc_base" in
        *[_-]*) pc_region=$(printf '%s' "${pc_base#*[_-]}" | tr '[:lower:]' '[:upper:]') ;;
        *) pc_region="" ;;
    esac
    case "$pc_region" in
        *[!A-Z0-9]*) printf '%s\n' "$pc_primary"; return 0 ;;
    esac
    if [ -n "$pc_region" ]; then
        printf '%s-%s\n' "$pc_primary" "$pc_region"
    fi
    case "$pc_primary:$pc_region" in
        zh: | zh:CN | zh:SG | zh:MY) printf 'zh-Hans\n' ;;
        zh:TW | zh:HK | zh:MO) printf 'zh-Hant\n' ;;
    esac
    printf '%s\n' "$pc_primary"
}

# Will the catalog be in place once this plan has run?
catalog_planned() {
    if [ -e "$i18n_dir/$1.conf" ]; then
        return 0
    fi
    if [ "$do_i18n" = no ]; then
        return 1
    fi
    if [ -n "$catalog_src" ] && [ -f "$catalog_src/$1.conf" ] && lang_selected "$1"; then
        return 0
    fi
    return 1
}

language_line=""
if [ -z "$locale_raw" ]; then
    language_line="no LC_ALL/LC_MESSAGES/LANG set, so the interface stays English"
else
    selected=""
    for candidate in $(print_candidates "$locale_raw"); do
        if catalog_planned "$candidate"; then
            selected=$candidate
            break
        fi
    done
    if [ -n "$selected" ]; then
        language_line="$locale_var=$locale_raw selects $i18n_dir/$selected.conf"
    else
        skipped=""
        for candidate in $(print_candidates "$locale_raw"); do
            if [ -n "$catalog_src" ] && [ -f "$catalog_src/$candidate.conf" ]; then
                skipped=$candidate
                break
            fi
        done
        if [ -n "$skipped" ]; then
            language_line="$locale_var=$locale_raw wants $skipped.conf, which this run does not install (--lang $skipped)"
        else
            language_line="$locale_var=$locale_raw selects English (no catalog for it)"
        fi
    fi
fi

# --- show the plan ----------------------------------------------------------

printf 'dun install plan\n'
report prefix "$opt_prefix"

case "$plan_binary" in
    install) report binary "$bin_dir/dun (from $dun_bin)" ;;
    inplace) report binary "$bin_dir/dun (already the installed one)" ;;
    skip) report binary "not installed (--no-binary)" ;;
esac

case "$plan_plugin" in
    install) report plugin "$syntect_dest (from $syntect_src)" ;;
    skip)
        if [ -n "$syntect_src" ]; then
            report plugin "not installed"
        fi
        ;;
esac

case "$plan_installed_config" in
    write) report config "$installed_config (write from dun --dump-config)" ;;
    keep) report config "$installed_config (keep; --force rewrites it)" ;;
    skip) report config "not written (--no-config)" ;;
esac
case "$plan_stanza" in
    inline) note "with plugin.syntect.* enabling the plugin" ;;
    append) note "append plugin.syntect.* to it (old one kept as .bak)" ;;
    print) note "plugin.syntect.* lines will be printed for you to add" ;;
esac

case "$plan_user_config" in
    write) report personal "$user_config (write an empty template for your own settings)" ;;
    keep) report personal "$user_config (keep; it is yours)" ;;
    skip)
        if [ "$sudo_note" = yes ]; then
            report personal "skipped: running as root, see the note below"
        fi
        ;;
esac

if [ "$do_i18n" = yes ]; then
    if [ "$plan_i18n_kept" -gt 0 ]; then
        report catalogs "$i18n_dir (install $plan_i18n_new, keep $plan_i18n_kept)"
    else
        report catalogs "$i18n_dir (install $plan_i18n_new)"
    fi
else
    report catalogs "not installed (--no-i18n)"
fi

report language "$language_line"

case "$path_action" in
    append) report path "append to $rc_file: $path_line" ;;
    print) report path "$bin_dir is not on your PATH; the line to add will be printed" ;;
    present)
        if [ -n "$rc_file" ] && [ -n "$path_line_dir" ]; then
            report path "$rc_file already mentions $path_line_dir"
        fi
        ;;
esac

if [ "$sudo_note" = yes ]; then
    note "running as root: no personal configuration is written. Afterwards,"
    note "as ${SUDO_USER:-your user}: scripts/install.sh --no-binary --no-i18n"
fi

printf '\n'

if [ "$dry_run" = yes ]; then
    printf 'Dry run: nothing was written.\n'
    exit 0
fi

if [ "$interactive" = yes ]; then
    if ! ask "Proceed?" yes; then
        printf 'Nothing was written.\n'
        exit 0
    fi
    printf '\n'
fi

# --- act --------------------------------------------------------------------

# The three lines that turn the installed host into a working plugin. The
# path is absolute because dun launches the command directly, with no shell
# and a cleared environment (docs/plugin-protocol.md).
syntect_stanza() {
    printf '\n# Syntax highlighting, installed by scripts/install.sh\n'
    printf 'plugin.syntect.command = %s\n' "$(abs_path "$syntect_dest")"
    printf 'plugin.syntect.trust = user-trusted-external\n'
    printf 'plugin.syntect.roles = syntax-highlight\n'
}

user_config_template() {
    cat <<EOF
# dun — your own configuration.
#
# This file is applied on top of the installed configuration, key by key:
#   $installed_config
# Anything you set here wins; anything you leave out keeps the installed
# value, and failing that dun's built-in default.
#
# \`dun --dump-config\` prints every key with its built-in default, and
# F6 in the editor shows which files are in force.
#
# theme = dark
# mouse.enabled = false
# key.edit.toggle_fold = Ctrl+X,F
EOF
}

printf 'dun install\n'
report prefix "$opt_prefix"

if [ "$plan_binary" = install ]; then
    ensure_dir "$bin_dir"
    copy_file "$dun_bin" "$bin_dir/dun" 755
    report binary "$bin_dir/dun (from $dun_bin)"
elif [ "$plan_binary" = inplace ]; then
    report binary "$bin_dir/dun (already the installed one)"
fi

if [ "$plan_plugin" = install ]; then
    ensure_dir "$bin_dir"
    copy_file "$syntect_src" "$syntect_dest" 755
    report plugin "$syntect_dest (from $syntect_src)"
fi

if [ "$plan_installed_config" = write ]; then
    ensure_dir "$share_dir"
    if [ -e "$installed_config" ]; then
        cp "$installed_config" "$installed_config.bak" || die "cannot write $installed_config.bak"
    fi
    if ! "$dun_bin" --dump-config > "$installed_config.tmp.$$"; then
        rm -f "$installed_config.tmp.$$"
        die "$dun_bin --dump-config failed"
    fi
    if [ "$plan_stanza" = inline ]; then
        syntect_stanza >> "$installed_config.tmp.$$"
    fi
    mv "$installed_config.tmp.$$" "$installed_config" || die "cannot write $installed_config"
    report config "$installed_config (written from dun --dump-config)"
elif [ "$plan_installed_config" = keep ]; then
    report config "$installed_config (kept)"
fi

case "$plan_stanza" in
    append)
        cp "$installed_config" "$installed_config.bak" || die "cannot write $installed_config.bak"
        syntect_stanza >> "$installed_config"
        report plugin "enabled in $installed_config (old one is $installed_config.bak)"
        ;;
    print)
        note "to enable the plugin, add to $installed_config:"
        syntect_stanza | sed 's/^/            /'
        ;;
esac

if [ "$plan_user_config" = write ]; then
    ensure_dir "$(dirname "$user_config")"
    user_config_template > "$user_config.tmp.$$" || die "cannot write $user_config"
    mv "$user_config.tmp.$$" "$user_config" || die "cannot write $user_config"
    report personal "$user_config (yours: set what you want to change here)"
elif [ "$plan_user_config" = keep ]; then
    report personal "$user_config (kept; it is yours)"
fi

if [ "$do_i18n" = yes ] && [ "$plan_i18n_new" -gt 0 ]; then
    ensure_dir "$i18n_dir"
    for src in "$catalog_src"/*.conf; do
        [ -f "$src" ] || continue
        tag=$(basename "$src")
        tag=${tag%.conf}
        lang_selected "$tag" || continue
        dest=$i18n_dir/$tag.conf
        if [ -e "$dest" ] && [ "$force" = no ]; then
            continue
        fi
        copy_file "$src" "$dest" 644
    done
fi
if [ "$do_i18n" = yes ]; then
    if [ "$plan_i18n_kept" -gt 0 ]; then
        report catalogs "$i18n_dir (installed $plan_i18n_new, kept $plan_i18n_kept)"
    else
        report catalogs "$i18n_dir (installed $plan_i18n_new)"
    fi
fi

report language "$language_line"

case "$path_action" in
    append)
        printf '\n# Added by dun scripts/install.sh\n%s\n' "$path_line" >> "$rc_file" ||
            die "cannot write $rc_file"
        report path "$rc_file (appended; open a new shell or source it)"
        ;;
    print)
        report path "$bin_dir is not on your PATH; add this line yourself:"
        note "$path_line"
        ;;
    present)
        if [ -n "$rc_file" ] && [ -n "$path_line_dir" ]; then
            report path "$rc_file already mentions $path_line_dir"
        fi
        ;;
esac

if [ "$sudo_note" = yes ]; then
    note "running as root: no personal configuration was written. As"
    note "${SUDO_USER:-your user}: scripts/install.sh --no-binary --no-i18n"
fi

if [ "$do_i18n" = no ]; then
    note "no catalogs copied; without them the interface is English"
fi

note "guide: docs/user-guide.md   every key: docs/configuration.md"
