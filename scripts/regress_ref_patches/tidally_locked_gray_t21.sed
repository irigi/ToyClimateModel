# P0 (2026-11-15) added the climatology maps v850 and t500 to this case (needed by
# tools/tidally_locked.py). Applied to the reference commit's copy of the case so both
# builds write the same files.
s/maps = \["ts", "tas", "precip", "evap", "pw", "olr", "asr", "t850", "u850"\]/maps = ["ts", "tas", "precip", "evap", "pw", "olr", "asr", "t850", "u850", "v850", "t500"]/
