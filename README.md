# multitail
A multi-platform file tailer that uses colors to differentiate different files. Multitail defaults to following files.

####Note, basically works. Displaying two logs from different log sources doesn't insert a newline between the logs. Going to put this project on hold to work on [multitail-lib](https://github.com/joelself/multitail-lib), which will probably end up being the back-end for multitail.

Usage: `mtail FILE [FILE]*`

In the future:
```shell
mtail # invoked with no arguments, will read arguments from './mtail.toml'
mtail [OPTIONS] FILE BACKGROUND_COLOR:FOREGROUND_COLOR:[ATTRIBUTE] [FILE BACKGROUND_COLOR:FOREGROUND_COLOR:[ATTRIBUTE]]*
-h, --help              : Print out this help
-c file, --config=file  : Get all command line args from 'file'
# LIST OUT POSSIBLE COLORS
# LIST OUT POSSIBLE ATTRIBUTES
```
Things to do:
- [ ] Add license file
- [x] Implement finding the last newline
- [ ] Actually handle errors
- [x] Seek to end when opening a file
- [ ] Write unit tests
- [x] Join all handles and exit?
- [ ] Implement a multifile writer for integration tests
- [ ] Write integration tests
- [ ] Read commands while running
- [ ] Read options from a config file
