(def assoc
     (lambda (id lst)
       (if (empty? lst)
           'none
           (let (pair (car lst)
                      key (car pair))
             (if (= key id)
                 (car (cdr pair))
                 (assoc id (cdr lst)))))))

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
    (do
      (print (list x (eval x env)))
      )))

(def ret-v
  (fn (r)
    (car r)))

(def ret-e
  (fn (r)
    (car (cdr r))))

(def ret
  (fn (val env)
    (list val env)))

(def seq-eval
  (fn (exprs env)
    (foldl
     (fn (expr last-ret)
       (let [last-env (ret-e last-ret)]
         (eval expr last-env)))
     (ret '() env)
     exprs)))

(def eval
  (fn (expr env)
    (cond
      (list? expr) (let [name (car expr)
                         r (cdr expr)]
                     (cond
                       (= name '+) (ret (call+ (map (lambda (e) (eval e env)) r)) env)
                       (= name 'def) (let [name (car r)
                                           e (car (cdr r))
                                           rs (eval e env)
                                           val (ret-v rs)]
                                       (ret val (cons (list name val) (ret-e rs))))
                       #t 'not-implemented))
      (keyword? expr) (ret (assoc expr env) env)
      #t expr
      )))


(test '(+ 1 2 3) '((test 1)))

'done
