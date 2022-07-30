use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use crate::config::{DEADLOCK_KIND_CNT, DEADLOCK_MUTEX, DEADLOCK_SEMAPHORE};

pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

// LAB5 HINT: you might need to maintain data structures used for deadlock detection
// during sys_mutex_* and sys_semaphore_* syscalls
pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let task_index = if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    };
    process_inner.deadlock_work[DEADLOCK_MUTEX][task_index as usize] = 1;
    task_index
}

// LAB5 HINT: Return -0xDEAD if deadlock is detected
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();

    task_inner.deadlock_info.deadlock_need = Some((DEADLOCK_MUTEX as u32, mutex_id as u32, 1));

    drop(task_inner);
    drop(cur_task);


    if process_inner.deadlock_enable {
        if process_inner.detect_will_deadlock() {
            let cur_task = current_task().unwrap();
            let mut task_inner = cur_task.inner_exclusive_access();
            task_inner.deadlock_info.deadlock_need = None;
            return -0xDEAD
        }
    }

    drop(process_inner);
    drop(process);
    mutex.lock();

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    let d = &mut task_inner.deadlock_info;
    d.deadlock_need = None;

    d.deadlock_allocation[DEADLOCK_MUTEX][mutex_id] += 1;

    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    let d = &mut task_inner.deadlock_info;

    d.deadlock_allocation[DEADLOCK_MUTEX][mutex_id] -= 1;
    drop(task_inner);
    drop(cur_task);

    mutex.unlock();
    0
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    process_inner.deadlock_work[DEADLOCK_SEMAPHORE][id] = res_count as u32;
    id as isize
}

pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();

    let d = &mut task_inner.deadlock_info;

    d.deadlock_allocation[DEADLOCK_SEMAPHORE][sem_id] -= 1;
    drop(task_inner);
    drop(cur_task);

    sem.up();
    0
}

// LAB5 HINT: Return -0xDEAD if deadlock is detected
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();

    task_inner.deadlock_info.deadlock_need = Some((DEADLOCK_SEMAPHORE as u32, sem_id as u32, 1));


    drop(task_inner);
    drop(cur_task);


    if process_inner.deadlock_enable && process_inner.detect_will_deadlock() {
        let cur_task = current_task().unwrap();
        let mut task_inner = cur_task.inner_exclusive_access();
        task_inner.deadlock_info.deadlock_need = None;
        return -0xDEAD
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);

    sem.down();

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    let d = &mut task_inner.deadlock_info;
    d.deadlock_need = None;

    d.deadlock_allocation[DEADLOCK_SEMAPHORE][sem_id] += 1;

    0
}

pub fn sys_condvar_create(_arg: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}

pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}

// LAB5 YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    match _enabled {
        0 => process_inner.deadlock_enable = false,
        1 => process_inner.deadlock_enable = true,
        _ => return -1,
    }
    0
}
