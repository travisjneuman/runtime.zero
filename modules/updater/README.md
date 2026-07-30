# Updater module

Development-only domain classifier for installed, manager-owned update evidence.
It maps caller-supplied synthetic records into the shared finding contract. It
does not discover packages, access a network, run managers, create action plans,
or execute updates. Missing installed/manager evidence is blocked. Core does not
install or execute this package.
