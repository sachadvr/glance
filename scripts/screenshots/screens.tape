# Drives glance in a real terminal (VHS) and saves the README screenshots. Run through ../screenshots.sh, which
# builds glance and the demo repository and exports DEMO_REPOSITORY.
#
# VHS takes a screenshot a moment after asking for it: each one is followed by a pause, or it would catch the
# keys typed next.

Output "docs/screenshots/.recording.gif"

Set Shell "bash"
Set FontSize 15
Set Width 1300
Set Height 720
Set Padding 18
Set Theme "TokyoNight"
Set TypingSpeed 0

Hide
Type 'cd "$DEMO_REPOSITORY" && clear' Enter
Show

# The branch browser: changes since the main line, the tests in their own section.
Type "glance" Enter
Sleep 2s
Type "2"
Sleep 1s
Screenshot "docs/screenshots/files.png"
Sleep 1s

# A diff with syntax colours and the changed words emphasised (the first open loads the grammars).
Type "jj"
Enter
Sleep 2s
Screenshot "docs/screenshots/diff.png"
Sleep 1s

# The branch's commits, each unfolded into its files.
Escape
Sleep 500ms
Type "c"
Sleep 500ms
Type "s"
Sleep 1s
Screenshot "docs/screenshots/commits.png"
Sleep 1s
Type "q"
Sleep 500ms

# As git's pager: a blame you can follow to the commit that wrote a line.
Hide
Type "clear" Enter
Show
Type "git -c core.pager=glance blame src/parser.rs" Enter
Sleep 2s
Type "jjjjjjjjjj"
Sleep 1s
Screenshot "docs/screenshots/blame.png"
Sleep 1s
Type "q"
Sleep 500ms
