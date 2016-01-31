# multitail
A multi-platform file tailer that uses colors to differentiate different files. Multitail defaults to following files.

####Note, doesn't actually work yet.

Usage: `mtail FILE [FILE]*`

In the future:
```shell
mtail # invoked with no arguments, will read arguments from './mtail.toml'
mtail [OPTIONS] FILE BACKGROUND_COLOR:FOREGROUND_COLOR:[ATTRIBUTE] [FILE BACKGROUND_COLOR:FOREGROUND_COLOR:[ATTRIBUTE]]*
-h, --help              : Print out this help
-n size, --names=size   : Show the filename truncated to 'size' on the left of each line in reversed colors
-c file, --config=file  : Get all command line args from 'file'
# LIST OUT POSSIBLE COLORS
# LIST OUT POSSIBLE ATTRIBUTES
```
Things to do:
- [ ] Add license file
- [ ] Implement finding the last newline
- [ ] Actually handle errors
- [ ] Seek to end when opening a file
- [ ] Write unit tests
- [ ] Join all handles and exit?
- [ ] Implement a multifile writer for integration tests
- [ ] Write integration tests
- [ ] Read commands while running
- [ ] Read options from a config file
