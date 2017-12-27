var addon = require('../native');
var assert = require('chai').assert;

function promisify(fn) {
  return (...args) => new Promise((resolve, reject) => {
    function cb(err, res) {
      if (err) {
        // Why does this not work from the context of the task callback?
        setImmediate(reject, err);
      } else {
        setImmediate(resolve, res);
      }
    }

    fn(...args.concat(cb));
  });
}

const addonAsync = Object.entries(addon).reduce((acc, [k, v]) => ({
  ...acc,
  [k] : promisify(v)
}), {});

describe('Task', function() {
  it('completes a successful task', function (done) {
    addon.perform_async_task((err, n) => {
      if (err) {
        done(err);
      } else if (n === 17) {
        done();
      } else {
        done(new Error("not 17 but: " + n));
      }
    });
  });

  it('completes a failing task', function (done) {
    addon.perform_failing_task((err, n) => {
      if (err) {
        if (err.message === 'I am a failing task') {
          done();
        } else {
          done(new Error("expected error message 'I am a failing task', got: " + err.message));
        }
      } else {
        done(new Error("expected task to fail, got: " + n));
      }
    });
  });

  it('completes with owned task', () => (
    addonAsync.perform_owned_task("World")
      .then(res => assert.equal(res, "Hello, World!"))
  ));

  it('completes with borrowed task', () => (
    addonAsync.perform_borrowed_task("World")
      .then(res => assert.equal(res, "Hello, World!"))
  ));

  it('completes with static borrowed task', () => (
    addonAsync.perform_borrowed_task_static()
      .then(res => assert.equal(res, "Hello, World!"))
  ));

  it('completes with static shortened borrowed task', () => (
    addonAsync.perform_borrowed_task_static_short()
      .then(res => assert.equal(res, "Hello, World!"))
  ));

  it('completes with forgot borrowed task', () => (
    addonAsync.perform_borrowed_task_forgot("World")
      .then(res => assert.equal(res, "Hello, World!"))
  ));
});
