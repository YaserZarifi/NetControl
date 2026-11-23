# Monitoring limitations

This foundation build collects no traffic. Blank values mean unavailable, not zero. Browser preview cannot measure Windows traffic or change firewall settings.

The next milestone will use Windows interface byte counters with monotonic elapsed time. Adapter discovery, virtual/VPN filtering, counter resets, sleep gaps, and pause/resume must be tested before reporting real totals. Interface counters include local network traffic and must not be described as ISP-billed internet consumption.

Per-application accounting requires a separate Windows proof of concept. ETW coverage, byte semantics, PID reuse, UDP/TCP, privileges, and performance must be validated before enabling attribution. Socket tables alone are insufficient. Unattributed bytes must remain visible. No historical usage before collection begins can be inferred.
