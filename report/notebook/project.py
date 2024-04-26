#!/usr/bin/env python
# coding: utf-8

# # Evaluation code for filter_dp

# In[1]:


import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
# import opendp as dp


# Load and clean dataset

# In[2]:


# Load input datasets
input_salaries = pd.read_csv('input/EmployeeSalaries.csv', names=["Department","Department_Name","Division","Gender","Base_Salary","Overtime_Pay","Longevity_Pay","Grade"])
input_students = pd.read_csv('input/StudentsPerformance.csv', names=["gender","race_ethnicity","parental_education","lunch","test_preparation","math_score","reading_score","writing_score"])

# Load output datasets
output_salaries = pd.read_csv('output/EmployeeSalaries.perturbed.csv', names=["time","Department","Department_Name","Division","Gender","Base_Salary","Overtime_Pay","Longevity_Pay","Grade"])
output_students = pd.read_csv('output/StudentsPerformance.perturbed.csv', names=["time","gender","race_ethnicity","parental_education","lunch","test_preparation","math_score","reading_score","writing_score"])

# Remove the last line from the "student" input/output files
input_students.drop(input_students.tail(1).index,inplace=True)
output_students.drop(output_students.tail(1).index,inplace=True)


# In[3]:


# Salaries dataset analysis
print("Salaries Dataset:")
print("Original Base_Salary - Mean: {:.2f}, Std: {:.2f}".format(input_salaries['Base_Salary'].mean(), input_salaries['Base_Salary'].std()))
print("Perturbed Base_Salary - Mean: {:.2f}, Std: {:.2f}".format(output_salaries['Base_Salary'].mean(), output_salaries['Base_Salary'].std()))

plt.figure(figsize=(8, 6))
plt.hist(input_salaries['Base_Salary'], bins=250, alpha=0.5, label='Original')
plt.hist(output_salaries['Base_Salary'], bins=250, alpha=0.5, label='Perturbed')
plt.xlabel('Base_Salary')
plt.ylabel('Frequency')
plt.legend()
plt.show()

# Students dataset analysis
print("\nStudents Dataset:")
for score in ['math_score', 'reading_score', 'writing_score']:
    print("Original {} - Mean: {:.2f}, Std: {:.2f}".format(score, input_students[score].mean(), input_students[score].std()))
    print("Perturbed {} - Mean: {:.2f}, Std: {:.2f}".format(score, output_students[score].mean(), output_students[score].std()))

plt.figure(figsize=(8, 6))
for score in ['math_score', 'reading_score', 'writing_score']:
    plt.hist(input_students[score], bins=100, alpha=0.5, label='Original ' + score)
    plt.hist(output_students[score], bins=100, alpha=0.5, label='Perturbed ' + score)
plt.xlabel('Score')
plt.ylabel('Frequency')
plt.legend()
plt.show()


# In[4]:


# Calculate the mean absolute error (MAE) for each perturbed attribute
mae_base_salary = np.mean(np.abs(input_salaries['Base_Salary'] - output_salaries['Base_Salary']))
mae_math_score = np.mean(np.abs(input_students['math_score'] - output_students['math_score']))
mae_reading_score = np.mean(np.abs(input_students['reading_score'] - output_students['reading_score']))
mae_writing_score = np.mean(np.abs(input_students['writing_score'] - output_students['writing_score']))

print(f"Mean Absolute Error (MAE) for Base_Salary: {mae_base_salary:.2f}")
print(f"Mean Absolute Error (MAE) for math_score: {mae_math_score:.2f}")  
print(f"Mean Absolute Error (MAE) for reading_score: {mae_reading_score:.2f}")
print(f"Mean Absolute Error (MAE) for writing_score: {mae_writing_score:.2f}")

# Calculate the mean squared error (MSE) for each perturbed attribute
mse_base_salary = np.mean((input_salaries['Base_Salary'] - output_salaries['Base_Salary'])**2)
mse_math_score = np.mean((input_students['math_score'] - output_students['math_score'])**2)
mse_reading_score = np.mean((input_students['reading_score'] - output_students['reading_score'])**2)  
mse_writing_score = np.mean((input_students['writing_score'] - output_students['writing_score'])**2)

print(f"\nMean Squared Error (MSE) for Base_Salary: {mse_base_salary:.2f}")  
print(f"Mean Squared Error (MSE) for math_score: {mse_math_score:.2f}")
print(f"Mean Squared Error (MSE) for reading_score: {mse_reading_score:.2f}")
print(f"Mean Squared Error (MSE) for writing_score: {mse_writing_score:.2f}")

# Calculate the root mean squared error (RMSE) for each perturbed attribute  
rmse_base_salary = np.sqrt(mse_base_salary)
rmse_math_score = np.sqrt(mse_math_score)
rmse_reading_score = np.sqrt(mse_reading_score)
rmse_writing_score = np.sqrt(mse_writing_score)

print(f"\nRoot Mean Squared Error (RMSE) for Base_Salary: {rmse_base_salary:.2f}")
print(f"Root Mean Squared Error (RMSE) for math_score: {rmse_math_score:.2f}") 
print(f"Root Mean Squared Error (RMSE) for reading_score: {rmse_reading_score:.2f}")
print(f"Root Mean Squared Error (RMSE) for writing_score: {rmse_writing_score:.2f}")

# Plot the distribution of errors for each perturbed attribute
fig, axs = plt.subplots(2, 2, figsize=(12, 8))
axs[0, 0].hist(input_salaries['Base_Salary'] - output_salaries['Base_Salary'], bins=250)  
axs[0, 0].set_title('Error Distribution for Base_Salary')
axs[0, 1].hist(input_students['math_score'] - output_students['math_score'], bins=50)
axs[0, 1].set_title('Error Distribution for math_score')
axs[1, 0].hist(input_students['reading_score'] - output_students['reading_score'], bins=50)  
axs[1, 0].set_title('Error Distribution for reading_score')
axs[1, 1].hist(input_students['writing_score'] - output_students['writing_score'], bins=50)
axs[1, 1].set_title('Error Distribution for writing_score')
plt.tight_layout()
plt.show()

