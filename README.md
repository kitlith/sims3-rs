# sims3_rs
*Command-Line Tools and Libraries for sims3 package files*
This project is licensed under the MIT license.

## `find_merged_cc`
This tool is designed for finding custom content on sims exported from the game,
but it may work with other types of merged package files in the future.

### Usage
```
find_merged_cc 0.2.0
Kitlith <kitlith@kitl.pw>

USAGE:
    find_merged_cc [FLAGS] <PACKAGE> <DIR>...

FLAGS:
    -v, --full       Print full paths instead of just the package filenames
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <PACKAGE>    Merged file that contains custom content
    <DIR>...     Directories to search for custom content in
```
Example: `find_merged_cc.exe my_sim.package "path/to/CC Magic/Content/Packages"`

Additionally, a batch file `find_merged_cc.bat` is provided so that you can drag
and drop your merged package file onto if you don't want to use the command-line
interface.

The batch file assumes that your custom content is in the default location for
CC Magic. If this is not the case, you can change
`%USERPROFILE%\Documents\Electronic Arts\CC Magic\Content\Packages`
to your own custom content path in the batch file.

### Limitations
This tool has not been tested with any kind of merged package file other than
exported sim package files. Please feel free to test this and leave an issue
report if you find any issues with it!

Patterns have a known possible false-positive case:
If a pattern is used on a piece of clothing AND installed separately,
then the pattern will be found even if only the clothing was in the merged file.

## `geom_tri_count`
This tool is designed to figure out the triangle counts within a package file.
It *will not* produce usable results when used with a package file that contains
multiple pieces of custom content.

### Usage
```
Usage: geom_tri_count [packages/directories]
```

You can provide multiple filenames and directories on the commandline,
and all of them will be checked for package files, recursively.

As the tool is designed for use w/ drag-and-drop, it'll display
`Press any key to continue` when it is finished, so that the terminal window
won't automatically close.

### Example
I have 4 package files in a folder called `packages`:
 - agnelid_Butterflysims127af_12.7Kedit.package
 - agnelid_NewseaMoonRiver_9.8KeditV1.package
 - ChazyNewseaJ141SexyBombAFVerB.package
 - PlumblobsPeggyzone122TFEF.package

If I run `geom_tri_count.exe packages` (or drag-n-drop the packages folder onto `geom_tri_count.exe`)
I get the following output:
```
agnelid_NewseaMoonRiver_9.8KeditV1.package -- Polys: [9825, 6857, 2788, 654]
agnelid_Butterflysims127af_12.7Kedit.package -- Polys: [12776, 8970, 2751, 459]
ChazyNewseaJ141SexyBombAFVerB.package -- Polys: [15277, 7051, 3852, 2197, 2, 1]
PlumblobsPeggyzone122TFEF.package -- Polys: [24248, 24248, 24248, 24248] (16 submeshes)
Press any key to continue
```
and the following contents in `sims3_geom_poly_count.csv` on my Desktop. (this file is overwritten on every run!)
```
Filename, Max Vertices, Max Polygons
"agnelid_NewseaMoonRiver_9.8KeditV1.package", 8624, 9825
"agnelid_Butterflysims127af_12.7Kedit.package", 9076, 12776
"ChazyNewseaJ141SexyBombAFVerB.package", 11356, 15277
"PlumblobsPeggyzone122TFEF.package", 16870, 24248
```
which should render as the following table in the spreadsheet application of your choice:
| Filename                                     | Max Vertices | Max Polygons |
|----------------------------------------------|-------------:|-------------:|
| agnelid_NewseaMoonRiver_9.8KeditV1.package   |         8624 |         9825 |
| agnelid_Butterflysims127af_12.7Kedit.package |         9076 |        12776 |
| ChazyNewseaJ141SexyBombAFVerB.package        |        11356 |        15277 |
| PlumblobsPeggyzone122TFEF.package            |        16870 |        24248 |

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
This tool tries to extract a name from a package file and then rename the package file to match.

WARNING: This tool was created for a specific purpose, maybe experimentation or something.
I don't remember. I can work with you if you want to make it better or more useful.

## `extract`
This tool extracts a single item from a DBPF into a separate file.
It is something I threw together to be able to inspect an item from within a hex editor,
for format exploration (because of unclear documentation).

### Usage

```
Usage: extract <package> <T:G:I> <output>
```

Example: `extract.exe PlumblobsPeggyzone122TFEF.package 15a1849:cb05b3:a0bad0bee0028400 weird.geom`

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
