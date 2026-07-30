# Cache module

Development-only ownership-aware cache classifier over caller-supplied synthetic
evidence. Only exact runtime-owned cache evidence may become a quarantine
candidate; manager/system/user evidence remains report-only and unknown
ownership is blocked. It does not scan, quarantine, clean, or delete. Core does
not install or execute it.
