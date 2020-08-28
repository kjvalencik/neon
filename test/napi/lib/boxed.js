const addon = require('../native');
const { expect } = require('chai');
const assert = require('chai').assert;

class Person {
  constructor(name) {
    this._person = addon.person_new(name);
  }

  greet() {
    return addon.person_greet(this._person);
  }
}

class RefPerson {
  constructor(name) {
    this._person = addon.ref_person_new(name);
  }

  greet() {
    return addon.ref_person_greet(this._person);
  }

  setName(name) {
    addon.ref_person_set_name(this._person, name);

    return this;
  }

  fail() {
    addon.ref_person_fail(this._person);
  }
}

describe('boxed', function() {
  it('can call methods', function () {
    const person = new Person('World');
    const greeting = person.greet();

    assert.strictEqual(greeting, 'Hello, World!');
  });

  it('can call methods wrapped in a RefCell', function () {
    const person = new RefPerson('World');
    const greeting = person.greet();

    assert.strictEqual(greeting, 'Hello, World!');
  });

  it('can mutate with ref cell', function () {
    const person = (new RefPerson('World')).setName('Universe');
    const greeting = person.greet();

    assert.strictEqual(greeting, 'Hello, Universe!');
  });

  it('should dynamically check borrowing rules', function () {
    assert.throws(() => (new RefPerson('World')).fail(), /BorrowMutError/);
  });

  it('should type check externals', function () {
    // `any::type_name` does not guarantee exact format
    // failed downcast to neon::types::boxed::JsBox<napi::js::boxed::Person>
    assert.throws(() => addon.person_greet({}), /failed downcast to.*JsBox.*Person/);
  });

  it('should type check dynamic type', function () {
    const unit = addon.external_unit();

    assert.throws(() => addon.person_greet(unit), /failed downcast/);
  });

  it('should be able to callback from another thread', function (cb) {
    addon.thread_callback(cb);
  });

  it('should be able to callback from multiple threads', function (cb) {
    const n = 4;
    const set = new Set([...new Array(n)].map((_, i) => i));

    addon.multi_threaded_callback(n, function (x) {
      if (!set.delete(x)) {
        console.log('huh?');
        cb(new Error(`Unexpected callback value: ${x}`));
      }

      if (set.size === 0) {
        cb();
      }
    });
  });

  it('should be able to use an async greeter', function (cb) {
    const greeter = addon.greeter_new('Hello, World!', function (greeting) {
      if (greeting === 'Hello, World!') {
        cb();
      } else {
        new Error('Greeting did not match');
      }
    });

    addon.greeter_greet(greeter);
  });
});