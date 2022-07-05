# Installing the CLI application

The CLI application currently does not have an installer but the standalone version can be used.

## Standalone

The standalone version is the application itself bundled into a Zip file. The application can be extracted and manually placed in a well-known location.

* [Downloads](https://github.com/chfoo/webaves/releases)

### Windows

If the application does not start because of a missing VC Runtime, install the [latest one](https://docs.microsoft.com/en-US/cpp/windows/latest-supported-vc-redist?view=msvc-170) (usually the x64 version).

#### Adding to PATH

These optional steps describes how to add a location to the `PATH` environment variable. The location should be the folder containing the application. This is done so you don't have to manually specify the full path to the application.

1. Create the folder for the application.
2. Download and unzip the application to the folder that you've created.
3. Right-click Start, click "System"
4. Click "Advanced system settings"
5. On the "Advanced" tab, click "Environment variables..."
6. On the user variables, if "Path" variable *exists*:

    1. Select the "Path" row.
    2. Click "New".
    3. Click "Browse...".
    4. Navigate to the directory that you created
    5. Click "OK"

7. On the user variables, if "Path" variable *does not* exist:

    1. Click "New..."
    2. Enter `Path` as variable name
    3. Click on "Browse directory..."
    4. Navigate to the directory that you created
    5. Click "OK"

8. Save the Environment variables by clicking on "OK"

### MacOS

The application can be placed directly in `/usr/local/bin`.

### Linux

The application can be placed in `$HOME/.local/bin` on systemd systems or `$HOME/bin` on `.profile` configurations.
