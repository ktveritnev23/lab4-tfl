import pandas as pd 
import numpy as np 

df = pd.read_csv("language_parser/test_valid.csv")

print(df.head())

window_size = 10
df['smoothed_valid_ns'] = df['Valid_Avg_Time_ns'].rolling(window=window_size, min_periods=1).mean()
df['smoothed_opt_ns'] = df['Valid_Opt_Time_ns'].rolling(window=window_size, min_periods=1).mean()

df_new = df[['Length','smoothed_valid_ns','smoothed_opt_ns']]

print(df_new.head())
df_new.to_csv("valid_smooth.csv")


df = pd.read_csv("language_parser/test_invalid.csv")

print(df.head())

window_size = 10
df['smoothed_invalid_ns'] = df['Invalid_Avg_Time_ns'].rolling(window=window_size, min_periods=1).mean()
df['smoothed_opt_ns'] = df['Invalid_Opt_Time_ns'].rolling(window=window_size, min_periods=1).mean()

df_new = df[['Length','smoothed_invalid_ns','smoothed_opt_ns']]

print(df_new.head())
df_new.to_csv("invalid_smooth.csv")