# Records a real terminal GIF of phpz: install vhs (https://github.com/charmbracelet/vhs) and run `vhs docs/assets/demo.tape`.
Output docs/assets/demo.gif
Set Shell "bash"
Set FontSize 16
Set Width 1000
Set Height 520
Set Padding 20
Set TypingSpeed 60ms

Type "phpz install 8.3" Enter
Sleep 6s
Type "phpz use 8.3" Enter
Sleep 1s
Type "cd api && phpz local 8.2" Enter
Sleep 1s
Type "php -v | head -1" Enter
Sleep 1.5s
Type "cd .. && php -v | head -1" Enter
Sleep 3s
