Testing color and modes:
for m in 0 1 2 3 4 7 8 9 ; do echo "\e[0m\n--- $m ---" ; for i in {0..255}; do printf "\e[${m}m"'\e[38;5;%dm%3d ' $i $i; (( (i-15) * ((i-15) % 36))) || printf '\e[0m\n'; done ; echo "" ; done

for r in {0..255..2} ; do for g in {0..255} ; do
echo -n "\033[38;2;$r;$g;$((255-g));48;2;0;0;0m0\e[0m"
done ; printf "\n" ; done
