import pandas as pd

# 健康数据
df1 = pd.DataFrame({'id': range(1000), 'title': [f'title_{i}' for i in range(1000)], 'content': [f'content_{i}' for i in range(1000)]})
df1.to_parquet('test_healthy.parquet')

# 问题数据（空值 + 重复）
df2 = pd.DataFrame({'id': [1]*300 + list(range(300,1000)), 'title': [f'title_{i}' for i in range(1000)], 'content': [None]*420 + [f'c{i}' for i in range(420,1000)]})
df2.to_parquet('test_issues.parquet')

print("Test data generated successfully!")
