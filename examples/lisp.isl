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
     (lambda (f init lst)
       (if (empty? lst)
           init
           (foldl f (f (car lst) init) (cdr lst)))))

(def map
     (lambda (f lst)
       (if (empty? lst)
           '()
           (cons (f (car lst)) (map f (cdr lst))))))

(def call+
     (lambda (args)
       (foldl + 0 args)))

(def test
  (fn (x env)
      (print (list x (eval x env)))
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


(def ret
  (fn (val env)
    (list val env)))

(def ret-v
  (fn (r)
    (car r)))

(def ret-e
  (fn (r)
    (car (cdr r))))


(def make-func
  (fn (args env body)
    (list 'func args env body)))

(def func-tag  (fn (func) (n 0 func)))
(def func-args (fn (func) (n 1 func)))
(def func-env  (fn (func) (n 2 func)))
(def func-body (fn (func) (n 3 func)))

(def func?
  (fn (func)
    (= 'func (func-tag func))))


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
    (if (empty? exprs)
      (ret '() env)
      (let [e (car exprs)
            remaining (cdr exprs)
            r (eval e env)
            recur-r (map-eval remaining (ret-e r))
            ]
        (do
          (ret (cons (ret-v r) (ret-v recur-r)) (ret-e recur-r)))
        )
      )
    )
  )

(def is-syscall?
  (fn (sys)
    (contains? '(print empty? car cdr odd? cons print list) sys)))

(def syscall-invoke
  (fn (sys args)
    (let [a0 (n 0 args)]
      (cond
        (= sys 'empty?) (empty? a0)
        (= sys 'car) (car a0)
        (= sys 'cdr) (cdr a0)
        (= sys 'odd?) (odd? a0)
        (= sys 'print) (print a0)
        #t (if (= (len args) 1)
             (error `(syscall-not-found ,sys ,args))
             (let [a1 (n 1 args)]
               (cond
                 (= sys 'cons) (cons a0 a1)
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
                       (= name '+) (ret (call+ (map (lambda (e) (ret-v (eval e env))) r)) env)
                       (= name 'def) (let [name (car r)
                                           e (car (cdr r))
                                           rs (eval e env)
                                           val (ret-v rs)]
                                       (ret val (cons (list name val) (ret-e rs))))
                       (= name 'do) (seq-eval r env)
                       (= name 'if) (let [pred (n 0 r)
                                          then (n 1 r)
                                          els (n 2 r)]
                                      (let [pred-r (eval pred env)]
                                        (eval (if (ret-v pred-r)
                                                then
                                                els)
                                              (ret-e pred-r))))
                       (or (= name 'fn)
                           (= name 'lambda)) (let [args (n 0 r)
                                                   body (n 1 r)]
                                               (ret
                                                (make-func args env body)
                                                env))
                       (= name 'let) (let [bindings (car r)
                                           body (n 1 r)
                                           names (filter-index (fn (a idx) (even? idx)) bindings)
                                           values (filter-index (fn (a idx) (odd? idx)) bindings)]
                                       (if (= (len names) (len values))
                                         (if (all keyword? names)
                                           (let [vals-r (map-eval values env)
                                                 new-env (append (zip names (ret-v vals-r)) (ret-e vals-r))
                                                 body-r (eval
                                                         body
                                                         new-env
                                                         )
                                                 ]
                                             (ret (ret-v body-r) env)
                                             )
                                           (error 'error-binding-names-arent-keywords))
                                         (error 'error-uneven-bindings-in-let)
                                         )
                                       )
                       (= name 'quote) (ret r env)
                       (is-syscall? name) (let [args-r (map-eval r env)
                                                args-v (ret-v args-r)
                                                new-env (ret-e args-r)]
                                            (ret (syscall-invoke name args-v) new-env))
                       #t (let [vs-r (map-eval expr env)
                                f (car (ret-v vs-r))
                                args (cdr (ret-v vs-r))]
                            (if (func? f)
                              (eval (func-body f) (append (zip (func-args f) args) (append (ret-e vs-r) (func-env f))))
                              (error `(error-cannot-apply-nonfunc ,f))
                              )
                            )
                       ))
      (keyword? expr) (ret (assoc expr env) env)
      #t (ret expr env)
      )))

(print (filter-index (fn (a idx) (odd? idx)) '(a b c d e f g h)))

(test '(+ 1 2 3) '((test 1)))

(test '(def test 123) '())

(test '(do (def test 123) (+ test 2)) '())

(test '(if #f 1 2) '())

(print (map-eval '((+ 1 2 3) (def x 2) x (+ x 1)) '()))

(print (map-eval '(1) '()))

(print (zip '(a b c) '(1 2 3)))

(test '((fn (x) (+ 1 x)) 1) '())

(test '(let (x 1 y 2) (+ x y)) '())

'done
