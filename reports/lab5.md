# 实验五报告

## 编程作业

1. 添加死锁相关数据结构，存取申请内存需要的内存
  
2. 对所有线程的资源进行计算判断是否产生死锁
  
3. 完善死锁相关系统调用
  

## 问答作业

1. 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？
  答：需要回收的资源有`TaskUserRess`，回收内存，回收`fd_table`，回收`子进程`相关的资源。`TaskControlBlock`可能在锁中被引用，但是不用回收，因为资源已经回收。
  
2. 对比以下两种 `Mutex.unlock` 的实现，二者有什么区别？这些区别可能会导致什么问题？
  答：第一种先解锁，然后弹出等待锁的任务，第二种判断是否由等待的任务，如果有则弹出，否则解锁。会导致死锁。
  

```rust
impl Mutex for Mutex1 {
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        mutex_inner.locked = false;
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        }
    }
}

impl Mutex for Mutex2 {
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
```