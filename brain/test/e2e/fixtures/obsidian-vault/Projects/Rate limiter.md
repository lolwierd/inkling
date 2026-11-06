# Rate limiter

We keep one counter per user and reset it every minute.

## Failure modes
* Clock skew between nodes
* A reset that runs late
+ Plus-sign bullets work too

### Numbers
1) Ten requests a second
2) A burst of fifty

~~~
redis-cli INCR user:42
~~~
