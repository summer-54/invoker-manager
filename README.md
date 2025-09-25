# invoker-manager
Server tool which keeps connections to invokers and distribute submissions between them.

## invoker-manager → testing-system
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
#### Examples:
```
TYPE VERDICT
SUBMISSION cc67b6ff-471b-b262-b6de-42d4c8e2fab1
VERDICT CE
MESSAGE Rust isn't C++
DATA
```
```
TYPE VERDICT
SUBMISSION cc67b6ff-471b-b262-b6de-42d4c8e2fab1
VERDICT OK
SUM 90
GROUPS 50 20 20 0
DATA
OK 0.1 54
OK 0.4 54
OK 0.9 54
TL 1.0 54
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
## invoker-manager ← testing-system
```
<uuid [16 bites]><test count [2 bites]><data>
```
## invoker-manager ←→ invoker

## verdicts

Standart verdicts and new one - SK = Skipped

You can find it in [invoker repository](https://github.com/summer-54/invoker).
