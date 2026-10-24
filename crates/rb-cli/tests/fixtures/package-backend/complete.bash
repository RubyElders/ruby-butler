RB_TEST_BINARY=$1
shim=$2
shift 2
rb() { "$RB_TEST_BINARY" "$@"; }
_init_completion() {
    words=("${COMP_WORDS[@]}")
    cword=$COMP_CWORD
    cur=${words[cword]}
    prev=${words[cword - 1]}
}
compopt() { :; }
eval "$shim"
COMP_WORDS=(rb "$@")
COMP_CWORD=$((${#COMP_WORDS[@]} - 1))
printf -v COMP_LINE '%q ' "${COMP_WORDS[@]}"
COMP_LINE=${COMP_LINE% }
COMP_POINT=${#COMP_LINE}
_rb_completion
printf '%s\n' "${COMPREPLY[@]}"
