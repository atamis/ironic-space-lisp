(def assert-eq
  (fn (a b)
    (if (= a b)
      nil
      (error (list 'assert-error a b)))))

(assert-eq true true)

(def assert-eval
  (fn (sexpr env expected)
    (let [res (eval sexpr env)]
      (if (= expected (ret-v res))
        nil
        (error (list 'assert-eval-error sexpr env expected (ret-v res)))))))

(def assert-eval-ret
  (fn (sexpr env expected)
    (let [res (eval sexpr env)]
      (if (= expected res)
        nil
        (error (list 'assert-eval-error sexpr env expected res))))))

(def foldl
  (fn (f init lst)
    (if (empty? lst)
      init
      (foldl f (f (car lst) init) (cdr lst)))))

(assert-eq (foldl + 0 '(1 2 3 4 5)) 15)
(assert-eq (foldl cons '() '(:a :b :c)) '(:c :b :a))

(def map
  (lambda (f lst)
          (if (empty? lst)
            '()
            (cons (f (car lst)) (map f (cdr lst))))))

(assert-eq (map (fn (x) (+ x 1)) '(1 2 3 4)) '(2 3 4 5))

(def zip
  (fn (a b)
    (if (= (len a) (len b))
      (if (empty? a)
        '()
        (cons (list (car a) (car b)) (zip (cdr a) (cdr b))))
      (error 'error-zip-lists-uneven))))

(assert-eq (zip '(a b c) '(1 2 3))
           '((a 1)
             (b 2)
             (c 3)))


(def take
  (fn (n lst)
    (if (= n 0)
      '()
      (cons (car lst) (take (- n 1) (cdr lst))))))

(assert-eq '(x 1) (take 2 '(x 1 y 2)))


(def after
  (fn (n lst)
    (if (= n 0)
      lst
      (after (- n 1) (cdr lst)))))

(assert-eq '() (after 2 '(y 2)))


(def group-by
  (fn (n lst)
    (if (empty? lst)
      '()
      (cons (take n lst) (group-by n (after n lst))))))

(assert-eq '((x 1) (y 2)) (group-by 2 '(x 1 y 2)))


(def filter
  (fn (f lst)
    (if (empty? lst)
      '()
      (let [a (car lst)
            recur (filter f (cdr lst))
            ]
        (if (f a)
          (cons a recur)
          recur)))))


(def filter-index
  (fn (f lst)
    (filter-index-internal f lst 0)))

(def filter-index-internal
  (fn (f lst n)
    (if (empty? lst)
      '()
      (let [a (car lst)
            recur (filter-index-internal f (cdr lst) (+ n 1))
            ]
        (if (f a n)
          (cons a recur)
          recur)))))

(def all
  (fn (f lst)
    (if (empty? lst)
      true
      (and (f (car lst)) (all f (cdr lst))))))


(def contains?
  (fn (lst a)
    (if (empty? lst)
      false
      (if (= a (car lst))
        true
        (contains? (cdr lst) a)))))

(def count-items
  (fn (val)
    (if (list? val)
      (if (empty? val)
        1
        (foldl + 1 (map count-items val)))
      1)))

(def ret
  (fn (val env)
    {:val val
     :env env
     :tag 'ret}))

(def ret-tag (fn (r) (get r :tag)))
(def ret-v (fn (r) (get r :val)))
(def ret-e (fn (r) (get r :env)))

(def ret?
  (fn (ret)
    (if (map? ret)
      (= 'ret (ret-tag ret))
      false)))

(def make-func
  (fn (args env body)
    {:tag 'func
     :args args
     :env env
     :body body}))

(def func-tag  (fn (func) (get func :tag)))
(def func-args (fn (func) (get func :args)))
(def func-env  (fn (func) (get func :env)))
(def func-body (fn (func) (get func :body)))

(def func-apply-args
  (fn (func env vals)
    (foldl
     (fn
       (pair env)
       (let [name (nth 0 pair)
             value (nth 1 pair)]
         (assoc env name value)))
     env
     (zip (func-args func) vals))))

(def func?
  (fn (func)
    (if (map? func)
      (= 'func (func-tag func))
      false)))


(def seq-eval
  (fn (exprs env)
    (foldl
     (fn (expr last-ret)
       (let [last-env (ret-e last-ret)]
         (eval expr last-env)))
     (ret '() env)
     exprs)))

(def map-eval
  (fn (exprs env)
    (do (if (= env true) (error (list 'env-is-true exprs)) 0)
        (if (empty? exprs)
          (ret '() env)
          (let [expr (car exprs)
                remaining (cdr exprs)
                r (eval expr env)
                recur-r (map-eval remaining (ret-e r))
                ]
            (ret (cons (ret-v r) (ret-v recur-r)) (ret-e recur-r)))))))

(def cond-eval
  (fn (clauses env)
    (if (empty? clauses)
      (ret 'incomplete-cond-use-true env)
      (let [clause (car clauses)
            pred (nth 0 clause)
            then (nth 1 clause)
            pred-r (eval pred env)
            next-env (ret-e pred-r)]
        (if (ret-v pred-r)
          (eval then next-env)
          (cond-eval (cdr clauses) next-env))))))

(def let-bindings-eval
  (fn (bindings env)
    (foldl
     (fn (binding env)
       (let [name (nth 0 binding)
             expr (nth 1 binding)
             expr-r (eval expr env)]
         (if (symbol? name)
           (assoc (ret-e expr-r) name (ret-v expr-r))
           ;;(cons (list name (ret-v expr-r)) (ret-e expr-r))
           (error `(local-binding-not-symbol ~name)))))
     env
     bindings)))

(def is-syscall?
  (fn (sys)
    (get '#{empty? car cdr odd? cons print list? + = symbol?
            first rest
            or - nth len append size}
         sys)))

(def syscall-invoke
  (fn (sys args)
    (let [a0 (nth 0 args)]
      (cond
        (= sys 'empty?) (empty? a0)
        (= sys 'first) (first a0)
        (= sys 'rest) (rest a0)
        (= sys 'car) (car a0)
        (= sys 'cdr) (cdr a0)
        (= sys 'odd?) (odd? a0)
        (= sys 'print) (do (print (list 'hosted a0)) a0)
        (= sys 'list?) (list? a0)
        (= sys 'symbol?) (symbol? a0)
        (= sys 'len) (len a0)
        (= sys 'size) (size a0)
        true (if (= (len args) 1)
               (error `(syscall-not-found-with-1-arg ~sys ~args))
               (let [a1 (nth 1 args)]
                 (cond
                   (= sys 'cons) (cons a0 a1)
                   (= sys '+) (+ a0 a1)
                   (= sys '-) (- a0 a1)
                   (= sys '=) (= a0 a1)
                   (= sys 'or) (or a0 a1)
                   (= sys 'nth) (nth a0 a1)
                   (= sys 'append) (append a0 a1)
                   true (error `(syscall-not-found ~sys ~args)))))))))



(def eval
  (fn (expr env)
    (cond
      (list? expr) (let [name (car expr)
                         r (cdr expr)]
                     (cond
                       (= name 'def) (let [name (car r)
                                           e (car (cdr r))
                                           rs (eval e env)
                                           val (ret-v rs)]
                                       (ret val
                                            (assoc (ret-e rs) name val)
                                            ;;(cons (list name val) (ret-e rs))
                                            ))
                       (= name 'do) (seq-eval r env)
                       (= name 'if) (let [pred (nth 0 r)
                                          then (nth 1 r)
                                          els (nth 2 r)]
                                      (let [pred-r (eval pred env)]
                                        (eval (if (ret-v pred-r)
                                                then
                                                els)
                                              (ret-e pred-r))))
                       (or (= name 'fn)
                           (= name 'lambda)) (let [args (nth 0 r)
                                                   body (nth 1 r)]
                           (ret
                            (make-func args env body)
                            env))
                       (= name 'let) (let [bindings (group-by 2 (car r))
                                           body (nth 1 r)
                                           local-env (let-bindings-eval bindings env)
                                           body-val (ret-v (eval body local-env))
                                           ]
                                       (ret body-val env))
                       (= name 'cond) (cond-eval (group-by 2 r) env)
                       (= name 'list) (map-eval r env)
                       (= name 'quote) (ret (car r) env)
                       (is-syscall? name) (let [args-r (map-eval r env)
                                                args-v (ret-v args-r)
                                                new-env (ret-e args-r)]
                                            (ret (syscall-invoke name args-v) new-env))
                       true (let [vs-r (map-eval expr env)
                                  f (car (ret-v vs-r))
                                  args (cdr (ret-v vs-r))
                                  local-bindings (func-apply-args f {} args)]
                              (if (func? f)
                                (ret
                                 (ret-v (eval (func-body f)
                                              (merge local-bindings
                                                     (merge (ret-e vs-r)
                                                            (func-env f)))))
                                 (ret-e vs-r))
                                (error `(error-cannot-apply-nonfunc ~f))))))
      (symbol? expr)
      (let [v (get env expr)]
        (if (= nil v)
          (error `(error-unbound-variable ~expr))
          (ret v  env)))
      true (ret expr env))))


(print (size '(1 2 3 4 5)))

(assert-eval 1 {} 1)

(assert-eval '(len '()) {} 0)

(assert-eval '(let (x 1 y 2) x) {} 1)

(assert-eval '1 {'test 1} 1)

(assert-eval '(do 1) {'test 1} 1)

(assert-eval '(cond true 1) {'test 1} 1)

(assert-eq (filter-index (fn (a idx) (odd? idx)) '(a b c d e f g h))
           '(b d f h))

;; This should either error, or be 6. Instead, it silently drops
;; the last arg and returns 3
;; (assert-eval '(+ 1 2 3) {'test 1} 6)

(assert-eval-ret '(def test 123) {} (ret 123 {'test 123}))

(assert-eval '(if false 1 2) {} 2)

(assert-eval '(do (def test 123) (+ test 2)) {} 125)

(assert-eval '(list 1 2 3) {} '(1 2 3))

(assert-eval '(quote asdfasdfasdf) {} 'asdfasdfasdf)

(assert-eq (map-eval '() {}) (ret '() {}))

(assert-eq (map-eval '(1) {}) (ret '(1) {}))

(assert-eq (map-eval '((+ 1 2)
                       (def x 2)
                       x
                       (+ x 1)) {})
           (ret '(3 2 2 3) {'x 2}))

(assert-eval '((fn (x) (+ 1 x)) 1) {} 2)

(assert-eval '(let (x 1 y 2) (+ x y)) {} 3)

(assert-eval '(let (x 1 y x) y) {} 1)

'done
