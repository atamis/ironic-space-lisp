(def assoc
  (fn (id lst)
    (if (empty? lst)
      (error `(assoc-not-found ,id))
      (let (pair (car lst)
                 key (car pair))
        (if (= key id)
          (car (cdr pair))
          (assoc id (cdr lst)))))))

(def assoc-contains?
  (fn (id lst)
    (if (empty? lst)
      #f
      (let (pair (car lst)
                 key (car pair))
        (if (= key id)
          #t
          (assoc-contains? id (cdr lst))))
      )
    )
  )

(def foldl
  (fn (f init lst)
    (if (empty? lst)
      init
      (foldl f (f (car lst) init) (cdr lst)))))

(def map
  (lambda (f lst)
          (if (empty? lst)
            '()
            (cons (f (car lst)) (map f (cdr lst))))))

(def test
  (fn (x env)
    (print (list x (ret-v (eval x env))))
    ))

(def zip
  (fn (a b)
    (if (= (len a) (len b))
      (if (empty? a)
        '()
        (cons (list (car a) (car b)) (zip (cdr a) (cdr b)))
        )
      (error 'error-zip-lists-uneven)
      )
    )
  )

(def take
  (fn (n lst)
    (if (= n 0)
      '()
      (cons (car lst) (take (- n 1) (cdr lst)))
      )
    )
  )

(def after
  (fn (n lst)
    (if (= n 0)
      lst
      (after (- n 1) (cdr lst))
      )
    )
  )

(def group-by
  (fn (n lst)
    (if (empty? lst)
      '()
      (cons (take n lst) (group-by n (after n lst)))
      )
    )
  )

(def filter
  (fn (f lst)
    (if (empty? lst)
      '()
      (let [a (car lst)
            recur (filter f (cdr lst))
            ]
        (if (f a)
          (cons a recur)
          recur
          )
        )
      )
    )
  )


(def filter-index
  (fn (f lst)
    (filter-index-internal f lst 0)
    )
  )

(def filter-index-internal
  (fn (f lst n)
    (if (empty? lst)
      '()
      (let [a (car lst)
            recur (filter-index-internal f (cdr lst) (+ n 1))
            ]
        (if (f a n)
          (cons a recur)
          recur
          )
        )
      )
    )
  )

(def all
  (fn (f lst)
    (if (empty? lst)
      #t
      (and (f (car lst)) (all f (cdr lst)))
      ))
  )


(def contains?
  (fn (lst a)
    (if (empty? lst)
      #f
      (if (= a (car lst))
        #t
        (contains? (cdr lst) a))
      )
    ))

(def count-items
  (fn (val)
    (if (list? val)
      (if (empty? val)
        1
        (foldl + 1 (map count-items val)))
      1
      )
    )
  )

(def ret
  (fn (val env)
    (list 'ret val env)))

(def ret-tag (fn (r) (nth 0 r)))
(def ret-v (fn (r) (nth 1 r)))
(def ret-e (fn (r) (nth 2 r)))

(def ret?
  (fn (ret)
    (if (list? ret)
      (= 'ret (ret-tag ret))
      #f
      )))

(def make-func
  (fn (args env body)
    (list 'func args env body)))

(def func-tag  (fn (func) (nth 0 func)))
(def func-args (fn (func) (nth 1 func)))
(def func-env  (fn (func) (nth 2 func)))
(def func-body (fn (func) (nth 3 func)))

(def func?
  (fn (func)
    (if (list? func)
      (= 'func (func-tag func))
      #f
      )))


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
    (do (if (= env #t) (error (list 'env-is-true exprs)) 0)
        (if (empty? exprs)
          (ret '() env)
          (let [expr (car exprs)
                remaining (cdr exprs)
                r (eval expr env)
                recur-r (map-eval remaining (ret-e r))
                ]
            (ret (cons (ret-v r) (ret-v recur-r)) (ret-e recur-r))
            )
          ))
    )
  )

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
          (cond-eval (cdr clauses) next-env)
          )
        )
      )
    ))

(def let-bindings-eval
  (fn (bindings env)
    (foldl
     (fn (binding env)
       (let [name (nth 0 binding)
             expr (nth 1 binding)
             expr-r (eval expr env)]
         (if (keyword? name)
           (cons (list name (ret-v expr-r)) (ret-e expr-r))
           (error `(local-binding-not-keyword ,name))
           )
         )
       )
     env
     bindings)
    ))

(def is-syscall?
  (fn (sys)
    (contains? '(empty? car cdr odd? cons print list? + = keyword? or - nth len append size) sys)))

(def syscall-invoke
  (fn (sys args)
    (let [a0 (nth 0 args)]
      (cond
        (= sys 'empty?) (empty? a0)
        (= sys 'car) (car a0)
        (= sys 'cdr) (cdr a0)
        (= sys 'odd?) (odd? a0)
        (= sys 'print) (do (print (list 'hosted a0)) a0)
        (= sys 'list?) (list? a0)
        (= sys 'keyword?) (keyword? a0)
        (= sys 'len) (len a0)
        (= sys 'size) (size a0)
        #t (if (= (len args) 1)
             (error `(syscall-not-found ,sys ,args))
             (let [a1 (nth 1 args)]
               (cond
                 (= sys 'cons) (cons a0 a1)
                 (= sys '+) (+ a0 a1)
                 (= sys '-) (- a0 a1)
                 (= sys '=) (= a0 a1)
                 (= sys 'or) (or a0 a1)
                 (= sys 'nth) (nth a0 a1)
                 (= sys 'append) (append a0 a1)
                 #t (error `(syscall-not-found ,sys ,args)))))
        )
      )
    ))



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
                                       (ret val (cons (list name val) (ret-e rs))))
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
                                       (ret body-val env)
                                       )
                       (= name 'cond) (cond-eval (group-by 2 r) env)
                       (= name 'list) (map-eval r env)
                       (= name 'quote) (ret (car r) env)
                       (is-syscall? name) (let [args-r (map-eval r env)
                                                args-v (ret-v args-r)
                                                new-env (ret-e args-r)]
                                            (ret (syscall-invoke name args-v) new-env))
                       #t (let [vs-r (map-eval expr env)
                                f (car (ret-v vs-r))
                                args (cdr (ret-v vs-r))
                                local-bindings (zip (func-args f) args)]
                            (if (func? f)
                              (ret
                               (ret-v (eval (func-body f) (append local-bindings (append (ret-e vs-r) (func-env f)))))
                               (ret-e vs-r)
                               )
                              (error `(error-cannot-apply-nonfunc ,f))
                              )
                            )
                       ))
      (keyword? expr) (ret (assoc expr env) env)
      #t (ret expr env)
      )))


(def exa '(x 1 y 2))

(print (take 2 exa))

(print (size '(1 2 3 4 5)))

(print (after 2 '(y 2)))

(print (group-by 2 exa))

(test '(let (x 1 y 2) x) '())

(test '1 '((test 1)))

(test '(cond #t 1) '((test 1)))

(print (filter-index (fn (a idx) (odd? idx)) '(a b c d e f g h)))

(test '(+ 1 2 3) '((test 1)))

(test '(def test 123) '())

(test '(if #f 1 2) '())

(test '(do (def test 123) (+ test 2)) '())

(test '(list 1 2 3) '())

(test '(quote asdfasdfasdf) '())

(print (map-eval '() '()))

(print (map-eval '(1) '()))

(print (map-eval '((+ 1 2 3)
                   (def x 2)
                   x
                   (+ x 1)) '()))

(print (zip '(a b c) '(1 2 3)))

(test '((fn (x) (+ 1 x)) 1) '())

(test '(let (x 1 y 2) (+ x y)) '())

(test '(let (x 1 y x) y) '())


'done
