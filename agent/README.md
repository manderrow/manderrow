# manderrow-agent

Injectable agent that handles redirecting output to IPC, injecting appropriate mod loaders, etc.

# Wine

When compiled to run under Wine, the agent will be named `manderrow-agent.dll.so` and will proxy IPC calls to a host shared library located by a command line argument.
