const list = [4,5,6];
const iterable = new Map([['a', 1], ['b', 2], ['c', 3]]);

for (let i of [1,2,3]) {
  const for_of_foo = document;
  const blah = list[i];
}

for (const [key, value] of iterable) {
  console.log(value);
}

for ((item) of list) {}
