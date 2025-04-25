# Changelog

All notable changes to this project will be documented in this file.

## 0.3.2

- Add SM packet retransmission logic.
  - However, if the acknowledge packet is lost, retransmission can cause vacant sessions at the server side.
    This is not a serious problem, so it is currently ignored.
- Update `likely` implementation, but not sure whether it works.