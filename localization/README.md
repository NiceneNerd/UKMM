# Localization

UKMM is now capable of localization for all the languages that Breath of the Wild is available for.

The goal is to make UKMM more accessible to those for whom English is not a first (or known)
language, so that users can understand the interface more easily, with less need to refer to
external tutorials.

## Localizing UKMM

Each localization file will be laid out like standard JSON, i.e.

`es.json`:
```
{
    "Generic_No": "No",
    "Generic_Yes": "Sí"
}
```

For this Spanish localization file, if the user were to select Spanish in the settings, all usages
of the localization key "Generic\_Yes" would be replaced with "Sí" and all usages of "Generic\_No"
would be replaced with "No" - their corresponding values in the `en.json` would be "Yes" and "No",
obviously.

## Key Usage

Each key is used in one or more specific places in the application. Those places are, as best as can
be, denoted here.

Note: Some keys may have curly braces in them `{}`. The curly braces and the text inside them must
not be changed, in order for the program to work. The text inside them is a description of what
the `{}` will be replaced with by UKMM. For example, `{mod_name}` will be replaced with a given
mod's name.

### Groups

The keys are split into groups, to make it easier to find them.

#### Busy

These keys are, generally, displayed when UKMM is doing something, and the interface is locked and
showing a progress modal.

```
Busy_Working: Title of the progress modal
Busy_Processing: The message shown inside the progress modal, right above the current step of
    whatever it's doing
```

#### Changelog

These keys are shown when there is an available UKMM update, generally inside the changelog wrapper
window.

```
Changelog_Bitcoin: Text on the button for copying NiceneNerd's bitcoin address
Changelog_Bitcoin_Copy: Tooltip for the button to copy NiceneNerd's bitcoin address
Changelog_Bitcoin_Copied: Message popup when the bitcoin button has been clicked
Changelog_New: Title at the top of the update modal window
Changelog_Subscribe: Text on the button to open NiceneNerd's Patreon
```

#### Deploy

These keys are generally shown in the Deploy tab. "Deploy", itself, is in the Tab section.

```
Deploy_Auto: Obsolete. Replaced by Settings_Platform_Deploy_Auto
Deploy_Auto_Failed: Shown at the bottom when deployment has failed and the user must click the
    Deploy button
Deploy_Method: Obsolete. Replaced by Settings_Platform_Deploy_Method
Deploy_NoConfig: Error message when someone tries to deploy without setting up their deployment
    settings for whichever platform (Switch, WiiU) they've selected
Deploy_OpenEmu: Shown on the button to run the emulator command
Deploy_OutputFolder: Obsolete. Replaced by Settings_Platform_Deploy_Output
```

#### Error

Most specific error messages are left in English at the current time. These keys correspond
to generic text inside any error window that may pop up

```
Error_Context: Clickable header to expand context information for errors UKMM knows about
Error_Details: Clickable header to expand code details on a generic error
Error_Label: Title of the error modal window
```

#### File Picker

These keys are used inside UKMM's custom file picker modal. Note: most operating systems use their
own file picker. UKMM's is only shown if the desktop environment doesn't have one UKMM can easily
use.

```
FilePicker_Back: Shown on the button to go back to the previous folder
FilePicker_Refresh: Shown on the button to reload the contents of the current folder
FilePicker_Up: Shown on the button to view the folder that contains the currently-shown folder
```

#### Generic

These keys are displayed in several places all over the application

```
Generic_Apply: Shown on the button in the Apply window on the bottom right, when changes have been
    made to the mod list
Generic_Cancel: Shown on buttons to tell UKMM to not perform some operation
Generic_Close: Shown on buttons to close windows/modals
Generic_Confirm: Shown on buttons to confirm that UKMM should perform some operation
Generic_Copied: Toast on the bottom right when something has been copied to the
    clipboard
Generic_Copy: Shown on buttons which, when clicked, copy something to the clipboard
Generic_Delete: Shown on buttons to show that UKMM will delete something, if clicked
Generic_Exit: Shown in the File menu, to exit the program
Generic_MarkdownSupported: Message showing that some Markdown style formatting is supported
    for a field in which a user can type
Generic_No: Shown on buttons to indicate a generic negative
Generic_OK: Shown on buttons to indicate the user acknowledges something
Generic_Reset: Shown on buttons to indicate that changes will be undone
Generic_Save: Shown on buttons to indicate that changes will be made permanent
Generic_Update: Shown on buttons or in context menus, for updating something
Generic_Yes: Shown on buttons to indicate the user accepts something UKMM has asked
```

#### Info

These keys are displayed on the Info tab

```
Info_Author: Header denoting a mod author's name
Info_Category: Header denoting the category a mod is in
Info_Description: Header denoting a mod's description
Info_Options: Header denoting which mod options have been enabled. No distinctions are made
    between required options, group options, and single options
Info_Options_None: Message denoting when no options have been enabled for the selected mod
Info_Priority: Header denoting order in which mods win/lose conflicts with each other. Higher
    priority number wins conflicts, if it matters
Info_Provide_Label: Title of a modal that pops up when UKMM asks for info about a mod that a user
    is trying to install.
Info_Provide_Message: Query in a modal, asking the user to provide information about a mod that they
    are trying to install. The mod, itself, did not come with information regarding its name,
    category, or description, and so the user must provide it.
Info_Manifest: Header for the list of files that a mod changes
Info_Manifest_BaseFiles: Clickable header to expand a list of files in the base game/update folders
    that the selected mod changes
Info_Manifest_DLCFiles: Clickable header to expand a list of files in the DLC folders that the
    selected mod changes
Info_Name: Header for a mod's name
Info_URL: Header for a mod's web address/URL
Info_Version: Header for the version number of a mod
```

#### Menu

These keys correspond to the toolbar at the top, and the options inside of it. Most options inside
the Window menu are under the "Tab" section.

```
Menu_File: File menu, contains the "open" and "exit" options
Menu_File_Open: Button to install a mod by opening a specific file
Menu_Help: Used for both the Help menu and the Help button to open the documentation
Menu_Help_About: Button to open the About modal, showing program details
Menu_Help_About_GUI: Header shown before the link to the egui code repo
Menu_Tools: Tools menu, contains various buttons related to storage locations and merge behavior
Menu_Tools_ConfigFolder: Button to open the folder containing UKMM's settings file
Menu_Tools_DeployFolder: Button to open the folder that UKMM deploys to for the current console mode
Menu_Tools_RefreshMerge: Button to delete the current profile's merged files and recreates them from
    scratch. Same as "remerge" in BCML
Menu_Tools_ResetPending: Button to rescan for changes between the merged profile files and the files
    in the output folder
Menu_Tools_StorageFolder: Button to open the folder containing mod and profile files
Menu_Window: Window menu, for showing/hiding various tabs
Menu_Window_Reset: Button to reset UKMM's layout to how it looked on first installation
```

#### Mod

These keys correspond to text shown 

```
Mod_Dev_Update: Button to open a file picker to update a mod's zip file to match the files inside
    the folder the user selects
Mod_Disable: Button to disable the selected mod
Mod_Enable: Button to enable the selected mod
Mod_Extract: Button to extract the selected mod's files
Mod_Move_End: Set the mod's priority to the highest number, bumping down all other mod priorities
    by 1
Mod_Move_Start: Set the mod's priority to 0, bumping up all other mod priorities by 1
Mod_Select_Title: Select a Mod
Mod_Selected_None: No mod selected
Mod_Send: Submenu for adding a mod to a profile other than the currently-displayed one
Mod_Uninstall: Button for uninstalling a mod from this profile
Mod_Uninstall_Confirmation: Question asking if the user wants to uninstall the selected mod. Uses
    {mod_name} to display the name of the mod being uninstalled
Mod_Unpack_Folder: Title of a file picker for selecting a folder to unpack a mod to
Mod_Update_Folder: Title of a file picker for selecting a folder to use to update the selected mod.
    Uses {mod_name} to display the name of the mod being updated
Mod_View: Button to open the folder containing the selected mod
```

#### Options

These keys are used either in the modal to modify options in a package the user is currently
making, or in a modal when installing a mod with options

```
Options_Add: Button for adding a new mod option to a selected group
Options_Configure: Title of the modal prompting the user to set mod options when packaging their mod
Options_Default: Header for a dropdown menu where the user selects which option will be used for
    the currently selected group by default (used for Exclusive groups only)
Options_Default_Enable: Checkbox denoting whether a mod option is enabled by default when the
    options menu pops up on mod installation (used for Multiple groups only)
Options_Desc: Header to a text box where the user enters the description of an option
Options_Folder: Header for a dropdown menu where the user selects which option folder will be used
    for the currently selected option
Options_Group_Add: Button to add a new group of options for the mod
Options_Group_Desc: Header to a text box where the user enters the description of an option group
Options_Group_Exclusive: Radio button to denote a group can only have 1 option selected
Options_Group_Multiple: Radio button to denote a group can have multiple options selected
Options_Group_Name: Header to a text box where the user enters the name of an option group
Options_Group_New: The name of an option group that a user creates, before they give it a custom
    name
Options_Group_Required: Checkbox denoting whether an option from this group must be installed
Options_Group_Required_Desc: Tooltip when hovering over the Options_Group_Required checkbox
Options_Group_Type: Header for the radio button group where the user selects if the group is a
    Multiple option group or an Exclusive option group
Options_Name: Header for the text box where the user enters the option name
Options_New: Button to add a new option to a group
Options_None: Radio button for an empty option in an Exclusive group, which a user can use when
    installing a mod to denote they don't want any of the selections in that exclusive group
Options_Required: Message displayed when a user has not selected at least one option from every
    required group
Options_Select: Title of the modal prompting the user to choose options for a mod they're installing
```

#### Package

These keys correspond to text shown directly inside the Package tab

```
Package_CrossPlatform: Checkbox to denote whether a mod can be installed on both WiiU and Switch
Package_CrossPlatform_Desc: Tooltip for the Package_CrossPlatform checkbox
Package_Dependencies: Button to select other installed mods that this package will depend on
Package_Finish: Button to finalize package properties and perform the package operation
Package_ManageOptions: Button to open the modal to manage mod options
Package_RootFolder: Header for a text box where the user selects the path to their mod's root folder
Package_Save_Title: Title of the file picker modal where the user selects where to save their mod
Package_Version_Desc: Tooltip for the text box where the user types in the version of the mod
    they're packaging
```

#### Pending

These keys correspond to the pop up in the bottom right when changes to the mod list need to be
applied

```
Pending_Changes: Title of the pop up
Pending_Files: Clickable header in the pop up, which shows all the files that will be changed when
    applying
```

#### Profile

These keys correspond to various messages related to profiles and the profile management menu

```
Profile_ActiveMods_1: Part 1 of the "# Mods / # Active" message in the Mods tab
Profile_ActiveMods_2: Part 2 of the "# Mods / # Active" message in the Mods tab
Profile_Added: Toast shown in the bottom right when mods have been sent to another profile. Uses
    {profile_name} to display the name of the destination profile
Profile_Delete_Confirmation: Question shown in a modal, asking for confirmation to delete a profile.
    Uses {profile_name} to display the name of the profile to delete
Profile_Duplicate: Button to copy a profile
Profile_Label: Title of the profile management modal
Profile_Manage: Tooltip shown when hovering the cursor over the button to open the profile
    management modal
Profile_New: Tooltip shown when hovering the cursor over the button to create a new profile
Profile_New_Label: Message shown above a text box telling the user to enter the name of a new
    profile
Profile_NoMods: Message shown when there are no mods in a profile
Profile_Rename: Button to rename a selected profile
Profile_Select: Message shown when hovering the cursor over the dropdown menu where the user can
    select the currently active profile
```

#### Settings

```
Settings_Changelog: Checkbox to select whether or not UKMM should show a summary when there is an
    available update
Settings_Changelog_Desc: Tooltip message for the Settings_Changelog button
Settings_Config_NX: Clickable header for the Switch settings section
Settings_Config_WiiU: Clickable header for the Wii U settings section
Settings_Config_WiiU_ImportCemu: Button to import emulator settings from Cemu
Settings_General: Clickable header for the general settings section
Settings_Language: Header for the dropdown box where the user can select UKMM's interface language
Settings_Language_Desc: Tooltip for the Settings_Language dropdown box
Settings_Migrate: Button to import settings from BCML
Settings_Mode: Radio group header, for whether UKMM is in Wii U or Switch mode
Settings_Mode_Desc: Tooltip for the Settings_Mode option
Settings_Mode_Switch: Radio button to set UKMM into Switch mode
Settings_Mode_WiiU: Radio button to set UKMM into Wii U mode
Settings_OneClick: Button to register your computer to redirect BCML 1-Click install links to UKMM
Settings_OneClick_Desc: Tooltip when hovering the cursor over the Settings_OneClick button
Settings_Platform_Deploy: Header for the deployment section of the settings
Settings_Platform_Deploy_Auto: Checkbox for the Auto Deploy option
Settings_Platform_Deploy_Auto_Desc: Tooltip for Settings_Platform_Deploy_Auto checkbox
Settings_Platform_Deploy_Emu: Header for the text box where the user can enter the command for
    running their game executable
Settings_Platform_Deploy_Emu_Desc: Tooltip for the Settings_Platform_Deploy_Emu setting
Settings_Platform_Deploy_Layout: Header for the option where the user selects whether or not UKMM
    adds a folder for itself. e.g. if the user enters "C:\mods" as their deploy folder, WithoutName
    will create "C:\mods\content" and WithName will create "C:\mods\BreathOfTheWild_UKMM\content"
Settings_Platform_Deploy_Layout_NX_Desc: Tooltip for the Settings_Platform_Deploy_Layout setting in
    the Switch config section
Settings_Platform_Deploy_Layout_NX_WithName: Radio button for the WithName option for the Switch
    config section (See Settings_Platform_Deploy_Layout)
Settings_Platform_Deploy_Layout_NX_WithoutName: Radio button for the WithoutName option for the
    Switch config setting (See Settings_Platform_Deploy_Layout)
Settings_Platform_Deploy_Layout_WiiU_Desc: Tooltip for the Settings_Platform_Deploy_Layout setting
    in the Wii U config section
Settings_Platform_Deploy_Layout_WiiU_WithName: Radio button for the WithName option for the Wii U
    config section (See Settings_Platform_Deploy_Layout)
Settings_Platform_Deploy_Layout_WiiU_WithoutName: Radio button for the WithoutName option for the
    Wii U config setting (See Settings_Platform_Deploy_Layout)
Settings_Platform_Deploy_Method: Header for the option where the user selects the method UKMM uses
    to deploy merged files to the output folder
Settings_Platform_Deploy_Method_Copy: Radio button for telling UKMM to copy all merged files
Settings_Platform_Deploy_Method_Desc: Tooltip for the Settings_Platform_Deploy_Method setting
Settings_Platform_Deploy_Method_HardLink: Radio button for telling UKMM to create shortcuts for all
    merged files
Settings_Platform_Deploy_Method_Symlink: Radio button for telling UKMM to create shortcuts to the
    base and DLC merged files
Settings_Platform_Deploy_Output: Text box for giving UKMM the folder path, where it will deploy
    merged mod files to
Settings_Platform_Deploy_Output_Desc: Tooltip for the Settings_Platform_Deploy_Output setting
Settings_Platform_Deploy_Rules: Checkbox for telling UKMM to write a rules.txt file to the output
    folder
Settings_Platform_Deploy_Rules_Desc: Tooltip for the Settings_Platform_Deploy_Rules setting
Settings_Platform_Dump: Header for the section of settings regarding where UKMM can find vanilla
    game files
Settings_Platform_Dump_DLC: Text box where the user can enter a path to the DLC files
Settings_Platform_Dump_DLC_NX_Desc: Tooltip for the Settings_Platform_Dump_DLC in the Switch section
Settings_Platform_Dump_DLC_WiiU_Desc: Tooltip for the Settings_Platform_Dump_DLC in the WiiU section
Settings_Platform_Dump_NX_Base: Text box where the user can enter a path to the combined base game
    and update files, only displayed in Switch mode
Settings_Platform_Dump_NX_Base_Desc: Tooltip for the Settings_Platform_Dump_NX_Base setting
Settings_Platform_Dump_WiiU_Base: Text box where the user can enter a path to the combined base game
    and update files, only displayed in Wii U mode
Settings_Platform_Dump_WiiU_Base_Desc: Tooltip for the Settings_Platform_Dump_WiiU_Base setting
Settings_Platform_Dump_Type: Radio button group label for selecting the format of the user's game
    dump
Settings_Platform_Dump_Type_Desc: Tooltip for the Settings_Platform_Dump_Type setting
Settings_Platform_Dump_Type_Unpacked: Radio button label for selecting that the game dump is
    unpacked loose files
Settings_Platform_Dump_Type_WUA: Radio button label for selecting that the game dump is a .wua file.
Settings_Platform_Dump_Update: Text box where the user can enter a path to the update files, only
    displayed in Wii U mode
Settings_Platform_Dump_Update_Desc: Tooltip for the Settings_Platform_Dump_Update setting
Settings_Platform_Dump_WUA: Text box where the user can enter a path to their .wua file, only
    displayed in Wii U mode
Settings_Platform_Dump_WUA_Desc: Tooltip for the Settings_Platform_Dump_WUA setting
Settings_Platform_Language: Dropdown menu header for selecting the language/region they use when
    playing BotW
Settings_Platform_Language_Desc: Tooltip for the Settings_Platform_Language setting
Settings_Saved: Toast that appears in the bottom right after settings have successfully be saved
Settings_SelectFolder_Cemu: Title of the file picker for selecting the folder that contains the
    Cemu executable
Settings_Storage: Header for the text box where the user can enter a path where UKMM will store mods
    and profiles
Settings_Storage_Desc: Tooltip for the Settings_Storage setting
Settings_Sys7z: Checkbox determining if UKMM will try to use a system installation of the 7zip
    program, instead of its internal 7zip code. UKMM's internal 7zip code is slower, but the system
    7zip can only be used if it's in the computer's PATH
Settings_Sys7z_Desc: Tooltip for the Settings_Sys7z setting
Settings_Theme: Dropdown menu for selecting the colors UKMM uses
Settings_Theme_Desc: Tooltip for the Settings_Theme setting
```

#### Tab

Titles of tabs. Also included in the Window menu at the top

```
Tab_Deploy: Deploy tab title
Tab_Info: Info tab title
Tab_Install: Install tab title
Tab_Log: Log tab title
Tab_Mods: Mods tab title
Tab_Package: Package tab title
Tab_Settings: Settings tab title
```

#### Update

Short message shown in the changelog window

```
Update_Available: The text inside the changelog window, informing the user an update is available 
```
