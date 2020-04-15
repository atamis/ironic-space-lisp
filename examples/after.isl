(def spawn
  (fn [f]
    (if (fork)
      (do (f) (terminate 'ok))
      #f)))
(spawn (fn [] (print 'hello-from-spawn)))

'done
