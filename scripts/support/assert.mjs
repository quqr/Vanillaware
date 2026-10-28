export class Report {
  constructor() {
    this.problems = [];
  }

  add(problem) {
    this.problems.push(problem);
  }

  check(condition, problem) {
    if (!condition) this.problems.push(problem);
  }

  equal(actual, expected, label) {
    if (actual !== expected) this.problems.push(`${label}: ${actual} != ${expected}`);
  }

  get failed() {
    return this.problems.length > 0;
  }

  print() {
    if (this.problems.length) {
      console.log(`\nFAIL\n${this.problems.join("\n")}`);
      return 1;
    }
    console.log("\nPASS");
    return 0;
  }
}
