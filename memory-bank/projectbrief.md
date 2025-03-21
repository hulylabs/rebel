# Rebel Project

Goals of the project is to create REBOL-inspired interpreter and VM for AI age and 2025. This means Rebel code will be written not only by Humans but also by AI agents.

Rebel VM will be used in many contexts such as:
* Shell -- implementing functions with you can find in any shell / operating system to manage files, directories, accessing network and OS functions. 
* Configuration Management System -- we may implement better alternative to Ansible, Chef, and similar configuration management on top of Rebel VM and functions.
* AI Augumentation -- environment where many AI agents, LLMs and humans collaborate, performing various operations simple by invoking Rebel programs.

Rebel will also extend REBOL with ideas which required to achieve project goals, such as introducing `process` -- long running function, and async interprocess comminication. These processes can run for days and weeks and persisted/recovered by VM. Here we should be better than similar frameworks like Temporal and others. We may introduce other necceary or helpful primitives which did not present in REBOL.
