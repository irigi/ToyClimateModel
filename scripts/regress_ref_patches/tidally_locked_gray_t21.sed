# P3 (2026-11-17): the gray scheme's latitude-dependent optical depth is off by default and
# off in the tidally locked cases (prototype defect 4). With the old code, tau_pole = tau_eq
# (6, the default) gives the same uniform τ₀ = tau_eq bit for bit.
/^scheme = "gray"$/a tau_pole = 6.0
