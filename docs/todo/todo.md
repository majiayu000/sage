高优先级问题                                                          
                                                                        
  1. 过多的 unwrap() 调用 (1,810 处)                                    
  - 生产环境中有 panic 风险                                             
  - 建议用 ? 操作符或 .context() 替换                                   
                                                                        
  2. 大量 println! 调用 (643 处)                                        
  - 应该使用 tracing 进行结构化日志                                     
  - 影响可观测性和性能                                                  
                                                                        
  3. 文件过大 (违反 200 行规范)                                         
  - subagent/types.rs (983 行)                                          
  - agent/base.rs (936 行)                                              
  - task_management.rs (931 行)                                         
  - interactive.rs (921 行)                                             
  - 建议拆分成更小的模块                                                
                                                                        
  性能优化                                                              
                                                                        
  4. 字符串分配 (1,494 处 clone/to_string)                              
  - LLM client 配置中有大量克隆                                         
  - 可以用引用和生命周期优化                                            
                                                                        
  5. 集合分配 (629 处)                                                  
  - 很多 Vec::new() 可以用 with_capacity() 预分配                       
                                                                        
  6. 锁竞争 (361 处 RwLock 操作)                                        
  - 可以用 DashMap 替换 HashMap + RwLock                                
  - 非异步场景可用 parking_lot::Mutex                                   
                                                                        
  代码一致性                                                            
                                                                        
  7. 静态初始化不统一                                                   
  - 有 lazy_static! (8 处) 和 once_cell (5 处)                          
  - 建议统一使用 once_cell                                              
                                                                        
  正面发现                                                              
                                                                        
  - 157 个文件有单元测试 ✓                                              
  - 357 个文件有文档注释 ✓                                              
  - 没有 TODO/FIXME 注释 ✓                                              
                                                                        
  ---                                                                   
  你想让我针对哪个方面进行具体优化？比如：                              
  1. 修复 unwrap() 调用                                                 
  2. 将 println! 迁移到 tracing                                         
  3. 拆分大文件                                                         
  4. 性能优化（预分配、减少克隆）                                       
                                                                        
> 3. 文件过大 (违反 200 行规范)                                         
                                                                        
                                                                        
  - subagent/types.rs (983 行)                                          
                                                                        
                                                                        
  - agent/base.rs (936 行)                                              
                                                                        
                                                                        
  - task_management.rs (931 行)                                         
                                                                        
                                                                        
  - interactive.rs (921 行)                                             
                                                                        
                                                                        
  - 建议拆分成更小的模块                                         