# Setting up ReNeo to start automatically

There are two ways to make ReNeo automatically start at logon. However, be aware that with both ways it's still *impossible to run ReNeo on the login screen* due to technical restrictions. This means you'll always have to enter your password using the native layout. If this is not acceptable, check if your preferred layout also provides a native driver DLL.

## Shortcut in startup directory

This method is easiest to setup but ReNeo won't work in applications running with administrative privileges.

Right click `reneo.exe` and select *Create shortcut*. Navigate to “C:\\Users\\[your user name]\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup” and copy the newly created shortcut into this directory.

## Using Task Scheduler

This method is a little more involved but it can run ReNeo with administrative privileges, meaning it'll also work in elevated applications. First, make sure there is no ReNeo shortcut in the startup directory, otherwise two instances run at the same time.

**Important:** A program that runs with administrative privileges must be installed in a folder that only administrators can change, e.g. `C:\Program Files\ReNeo`. If ReNeo runs elevated from a folder like `C:\Users\[USER]\ReNeo`, any program you start could replace `reneo.exe` or one of its files, and would then run with administrative privileges at your next logon. ReNeo shows a warning at startup in this case. Your settings are stored in `%APPDATA%\ReNeo` when ReNeo can't write to its own folder.

Use the start menu search to open the “Task Scheduler”. Create a new task called “ReNeo” with the following settings:

![Task Scheduler general tab](task_scheduler_general.png "Task Scheduler general tab")

Add a trigger for *At log on* and choose your user.

![Task Scheduler trigger tab](task_scheduler_triggers.png "Task Scheduler trigger tab")

As the action choose *Start a program* with the location of the ReNeo executable.

![Task Scheduler actions tab](task_scheduler_actions.png "Task Scheduler actions tab")

If you're on a laptop it's important to disable the highlighted power saving setting.

![Task Scheduler conditions tab](task_scheduler_conditions.png "Task Scheduler conditions tab")

Finally, under *Settings* allow ReNeo to run for longer than three days.

![Task Scheduler settings tab](task_scheduler_settings.png "Task Scheduler settings tab")