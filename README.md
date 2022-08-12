# Rust_Logger 
## A git-like utility for easily tracking file changes

My past experiences with computational experiments - in AI, physics, engineering, etc - have shown that it is very easy to lose track of what things have changed in your configs, code, or input data. At the start of a new moloecular dynamics project in mid-2022 I have decided that it is worth investing some time into creating a system which can effortlessly log any changes that might occur in a project.

You might ask "well why not use git?". You certainly could use git to track these types of changes but git is more meant for more significant code-base changes that are more or less permanent. In computational science you might want to only change 1 or 2 variables and see how that changes the result. This can quickly generate thousands of git commits that would be impossible to keep track of or analyze using vanilla git or github. As computational scientists we need to easily visualize the relationships between potentially thousands or millions of experimental inputs and outputs. This is what has motivated the current project.

## How it works
Once you have `log_server` set up somewhere and all the database/configuration settings are appropriately set, we can now begin using the `log_client`. 

The `log_client` manifests itself as a command-line tool called `log`. We first need to generate an account with our database. This account registration is triggered automatically the first time you use `log`:

![Alt text](imgs/registration.png)