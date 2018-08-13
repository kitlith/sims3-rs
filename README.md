# sims3_rs
*Command-Line Tools and Libraries for sims3 package files*
This project is licensed under the MIT license.

## `find_merged_cc`
This tool is designed for finding custom content on sims exported from the game,
but it may work with other types of merged package files in the future.

### Usage
Example: `find_merged_cc.exe my_sim.package "path/to/CC Magic/Content/Packages"`

Additionally, a batch file `find_merged_cc.bat` is provided so that you can drag
and drop your merged package file onto if you don't want to use the command-line
interface.

The batch file assumes that your custom content is in the default location for
CC Magic. If this is not the case, you can change
`%USERPROFILE%\Documents\Electronic Arts\CC Magic\Content\Packages`
to your own custom content path in the batch file.

### Limitations
For the moment, it can not find patterns on sims. It has not been tested with
any kind of merged package file other than sim package files. Please feel free
to test this and leave an issue report if you find issues with it!

## `dump_package_pngs`
This tool is designed to extract all png images from a package file.

WARNING: This tool was written as a kind of quick test, so it's unfinished and
may disappear or be heavily changed at any time.

### Usage
Example: `dump_package_pngs.exe necklace.package`

Images will be placed in the current directory.

Additionally, you should be able to just drag and drop a package file on top of
the exe, this should cause the images to be placed in the same directory of the
executable.

### Limitations
The output filename is based on the Instance ID of the image tag, which makes it
hard to read and may cause issues if two images share the same Instance ID but
are of different Resource Types.

## `package_names`
This tool is designed to extract a name map of names to Instance IDs.

WARNING: This tool is completely untested, and will most likely not remain in
the future. It is useless in its current state. However, I wanted to make sure
to keep the code around for the future. Don't use this!

# Developers
So, this was designed to be a rust library for doing stuff with sims3 package
files. However, I have not published it on crates.io or anything because I want
to finalize the API and crate name(s), as well as actually finish the library
before I claim any names. (This is also my first rust project, so...)
Please suggest improvements to the api and such?

## Notes on find_merged_cc limitations
At the moment, all I am doing is checking Type+Group+Instance IDs on specific
tags. Specifically, CASP, TONE (of the skin variety), and OBJD (in case someone
wants to use this for other types of merged package files). Checking image tags
or resource XML tags in this manner generates false-positives. This is why
patterns are not supported.

## TODO
 - [ ] Finish adding all resource types to the ResourceType enum.
 - [ ] Move the refpack decompression (and eventually compression) into its own crate.
 - [ ] How do I cleanly expose ResourceType dependent functionality?
 - [ ] Rename the crate? Possibly to `dbpf` or `sims3-dbpf`.
 - [ ] Move the binaries into their own crate so that their dependencies don't pollute library dependencies.
 - [ ] Merge USAGE.txt into README.md?
