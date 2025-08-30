# invoker-manager
Server tool which keeps connections to invokers and distribute submissions between them.

## invoker-manager -> testing-system
### Submission verdict
```
TYPE VERDICT
SUBMISSION <submission-uuid>
VERDICT <verdict>
// if <verdict> == OK {
SUM <sum>
GROUPS <points for 0 group> ... <points for n group>
// } else {
MESSAGE <error message>
// }
DATA
<verdict> <time> <memory> // 1 test result
...
<verdict> <time> <memory> // n test result
```
#### Example:
```
TYPE VERDICT
SUBMISSION cc67b6ff-471b-b262-b6de-42d4c8e2fab1
VERDICT CE
MESSAGE Rust isn't C++
DATA
WA 0.1 54
```

### Test verdict
```
TYPE TEST
SUBMISSION <submission-uuid>
TEST <test number>
VERDICT <verdict>
DATA
<data>
```
## invoker-manager <- testing-system

in progress

## invoker-manager <-> invoker

You can find it in [invoker repository](https://github.com/summer-54/invoker).
