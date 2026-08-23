use alchemist::{compile_source, run_source, vm::Vm};

#[test]
fn factorial_recursive_prints_120() {
    let src = r#"
        fn fact(n) {
            if (n <= 1) {
                return 1;
            } else {
                return n * fact(n - 1);
            }
        }
        print(fact(5));
    "#;
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["120"]);
}

#[test]
fn fibonacci_number() {
    let src = r#"
        fn fib(n) {
            if (n < 2) {
                return n;
            } else {
                return fib(n - 1) + fib(n - 2);
            }
        }
        print(fib(10));
    "#;
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["55"]);
}

#[test]
fn while_loop_sums_one_to_ten() {
    let src = r#"
        let sum = 0;
        let i = 1;
        while (i <= 10) {
            sum = i + sum;
            i = i + 1;
        }
        print(sum);
    "#;
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["55"]);
}

#[test]
fn arithmetic_precedence() {
    let src = "print(2 + 3 * 4);";
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["14"]);
}

#[test]
fn if_else_branch_selection() {
    let src = r#"
        let x = 7;
        if (x > 10) {
            print("big");
        } else {
            print("small");
        }
    "#;
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["small"]);
}

#[test]
fn function_with_parameters() {
    let src = r#"
        fn add(a, b) {
            return a + b;
        }
        print(add(3, 4));
    "#;
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["7"]);
}

#[test]
fn boolean_and_string_values() {
    let src = r#"
        let ok = true && false;
        print(ok);
        print("hi " + "there");
    "#;
    let out = run_source(src).expect("program should run");
    assert_eq!(out, vec!["false", "hi there"]);
}

#[test]
fn undefined_variable_is_compile_error() {
    let src = "print(missing);";
    let err = compile_source(src).expect_err("should fail to compile");
    let msg = err.to_string();
    assert!(msg.contains("undefined variable"), "unexpected message: {}", msg);
}

#[test]
fn arity_mismatch_is_compile_error() {
    let src = r#"
        fn add(a, b) { return a + b; }
        print(add(1));
    "#;
    let err = compile_source(src).expect_err("should fail to compile");
    let msg = err.to_string();
    assert!(msg.contains("expects 2 argument"), "unexpected message: {}", msg);
}

#[test]
fn division_by_zero_is_clean_runtime_error() {
    let src = "print(1 / 0);";
    let chunk = compile_source(src).expect("should compile fine");
    let mut machine = Vm::new(&chunk);
    let err = machine.run().expect_err("should fail at runtime");
    assert!(err.to_string().contains("division by zero"), "unexpected message: {}", err);
}

#[test]
fn deeply_nested_parens_is_parse_error_not_crash() {
    let mut src = String::from("print(");
    for _ in 0..5000 {
        src.push('(');
    }
    src.push('1');
    for _ in 0..5000 {
        src.push(')');
    }
    src.push_str(");");
    let result = compile_source(&src);
    assert!(result.is_err(), "deeply nested parens should be rejected, not overflow the stack");
}
